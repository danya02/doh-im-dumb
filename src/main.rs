use std::{
    fs::File,
    io::{self, BufReader},
    time::Duration,
};

use async_trait::async_trait;

use hickory_resolver::{
    config::{ResolverConfig, ResolverOpts},
    error::ResolveErrorKind,
    proto::{
        op::{Header, MessageType, OpCode, ResponseCode},
        rr::{Record, RecordType},
    },
    Name, TokioAsyncResolver,
};
use hickory_server::{
    authority::MessageResponseBuilder,
    server::{Request, RequestHandler, ResponseHandler, ResponseInfo},
    ServerFuture,
};
use rustls::{Certificate, PrivateKey};
use tokio::{net::TcpListener, time::timeout};
use tracing::{error, field::display, warn, Span};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let mut server = ServerFuture::new(DnsServer::new().await);
    let listener = TcpListener::bind("0.0.0.0:5443")
        .await
        .expect("Failed to listen to TLS port");

    let file = File::open("./secrets/crt.pem").unwrap();
    let mut reader = BufReader::new(file);
    let certs = rustls_pemfile::certs(&mut reader).unwrap();
    let certs = certs.into_iter().map(Certificate).collect();

    let file = File::open("secrets/key.pem").unwrap();
    let mut reader = BufReader::new(file);
    let mut keys = rustls_pemfile::ec_private_keys(&mut reader).unwrap();
    let key = match keys.len() {
        0 => panic!("No PKCS8-encoded private key found in file"),
        1 => PrivateKey(keys.remove(0)),
        _ => panic!("More than one PKCS8-encoded private key found in file"),
    };

    server
        .register_https_listener(listener, Duration::from_secs(120), (certs, key), None)
        .unwrap();

    server.block_until_done().await.unwrap()
}

pub struct DnsServer {
    resolver: TokioAsyncResolver,
}

impl DnsServer {
    pub async fn new() -> Self {
        Self {
            resolver: TokioAsyncResolver::tokio(
                ResolverConfig::cloudflare(),
                ResolverOpts::default(),
            ),
        }
    }

    async fn lookup<R: ResponseHandler>(
        &self,
        request: &Request,
        mut response_handle: R,
    ) -> io::Result<ResponseInfo> {
        let info = request.request_info();
        let r#type = info.query.query_type();
        if r#type == RecordType::AXFR {
            warn!("refused to handle {} query", r#type);
            let response = MessageResponseBuilder::from_message_request(request)
                .error_msg(info.header, ResponseCode::Refused);

            return response_handle.send_response(response).await;
        }

        match self.query(Name::from(info.query.name()), r#type).await {
            Ok(answers) => {
                let mut header = Header::response_from_request(info.header);
                header.set_authoritative(true);
                let response = MessageResponseBuilder::from_message_request(request).build(
                    header,
                    answers.iter(),
                    [].iter(),
                    [].iter(),
                    [].iter(),
                );

                response_handle.send_response(response).await
            }
            Err(code) => {
                let response = MessageResponseBuilder::from_message_request(request)
                    .error_msg(info.header, code);
                response_handle.send_response(response).await
            }
        }
    }

    async fn query(&self, name: Name, r#type: RecordType) -> Result<Vec<Record>, ResponseCode> {
        let authority = name.clone();
        // {
        //     if authority.num_labels() < 2 {
        //         // For now, we're not using this server for a TLD.
        //         return Err(ResponseCode::Refused);
        //     }
        // }

        let mut rsp = Vec::new();
        match self.resolver.lookup(authority, r#type).await {
            Ok(answers) => {
                // Add the resolver's answers to our response and return it.
                rsp.extend(answers.record_iter().cloned());
                return Ok(rsp);
            }
            Err(err) => match err.kind() {
                // The `rsp` might contain CNAME records we've gathered up to this point,
                // but if the ultimate target doesn't resolve, we probably shouldn't send these.
                ResolveErrorKind::NoRecordsFound { .. } => return Ok(Vec::new()),
                _ => {
                    warn!(%name, ?r#type, "resolver error while flattening: {err}");
                    return Err(ResponseCode::ServFail);
                }
            },
        }
    }
}

#[async_trait]
impl RequestHandler for DnsServer {
    #[tracing::instrument(skip(self, response_handle), fields(
        otel.kind = "server", otel.name, dns.op_code, dns.question.r#type, dns.question.name,
        net.transport, dns.protocol, net.sock.peer.addr, net.protocol.name
    ))]
    async fn handle_request<R: ResponseHandler>(
        &self,
        request: &Request,
        mut response_handle: R,
    ) -> ResponseInfo {
        let op_code = request.op_code();
        let QUERY_TIMEOUT = Duration::from_secs(3);

        let span = Span::current();
        span.record("dns.op_code", display(op_code));
        span.record("dns.proto", request.protocol().to_string());
        // TODO Use const after upgrading to opentelemetry-semantic-conventions 1.11
        span.record("net.sock.peer.addr", request.src().ip().to_string());
        span.record("net.protocol.name", "dns");

        let result = match request.message_type() {
            // TODO think about threading query lookups for multiple lookups, this could be a huge improvement
            //  especially for recursive lookups
            MessageType::Query if op_code == OpCode::Query => {
                span.record("dns.question.type", display(request.query().query_type()))
                    .record("dns.question.name", display(request.query().name()));
                let future = self.lookup(request, response_handle);
                match timeout(QUERY_TIMEOUT, future).await {
                    Ok(Ok(i)) => Ok(i),
                    Ok(Err(e)) => {
                        error!("error sending response: {}", e);
                        Err(ResponseCode::ServFail)
                    }
                    Err(_) => {
                        error!(?request, "request handler timed out");
                        Err(ResponseCode::ServFail)
                    }
                }
            }
            mtype => {
                let code = match mtype {
                    MessageType::Query => {
                        warn!("unimplemented op code: {:?}", op_code);
                        ResponseCode::NotImp
                    }
                    MessageType::Response => {
                        warn!("got a response as a request from id: {}", request.id());
                        ResponseCode::FormErr
                    }
                };

                let response = MessageResponseBuilder::from_message_request(request);
                response_handle
                    .send_response(response.error_msg(request.header(), code))
                    .await
                    .map_err(|e| {
                        error!("error sending response: {}", e);
                        ResponseCode::ServFail
                    })
            }
        };

        let err = match result {
            Ok(info) => return info,
            Err(e) => e,
        };

        error!("request failed: {}", err);
        let mut header = Header::new();
        header.set_response_code(ResponseCode::ServFail);
        header.into()
    }
}
