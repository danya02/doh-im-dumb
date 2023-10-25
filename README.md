# D'oH, I'm DUMB 😩

A DNS over HTTPS server that's plain dumb.

It takes incoming DNS requests and forwards them to a UDP DNS server,
returning whatever comes back.

It's intended to take advantage of Encrypted Client Hello features in browsers
that can retrieve HTTPS records from DoH servers only;
this is dumb on its own 🙃

At the moment, the app runs on HTTP;
you need to bring your own TLS termination.