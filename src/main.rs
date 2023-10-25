use axum::{
    response::{IntoResponse, Response},
    routing::post,
    Router,
};

#[tokio::main]
async fn main() {
    let app = Router::new().route("/", post(resolve_dns));

    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn resolve_dns() -> Response {
    "todo".into_response()
}
