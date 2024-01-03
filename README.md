# D'oH, I'm DUMB 😩

A DNS over HTTPS server that's plain dumb.

It takes incoming DNS requests and forwards them to a traditional UDP-based DNS server,
returning whatever comes back.

It's intended to take advantage of Encrypted Client Hello features in browsers
that can retrieve the records needed for ECH from DoH servers only (like Firefox as of version 121, for example);
this is dumb on its own 🙃

> [!CAUTION]
> There is a privacy risk associated with using this:
> any passive MITM can see the DNS queries being performed with this,
> and an active MITM may also replace the EDNS records with their own,
> **negating the security** of DNS-over-HTTPS.
> In my setup, this is acceptable, because my MITM only blocks connections based on the SNI field,
> and doesn't seem to check or modify DNS traffic at all;
> however, make sure to check the implications before deploying it in your network. 

It uses `crt.pem` and `key.pem` in the `secrets` directory to perform TLS termination.
However, in my setup, there is also a Traefik instance that sits in front of everything;
because of this, there are self-signed TLS certificates baked into the container image,
and a ServersTransport is set up to not verify them (you could also trust them explicitly in Traefik).
