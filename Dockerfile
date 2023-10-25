FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /app

FROM chef AS planner
RUN mkdir /common
COPY . /app
WORKDIR /app
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
RUN rustup target add x86_64-unknown-linux-musl
WORKDIR /app
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json --target=x86_64-unknown-linux-musl
# Build application
COPY . .
RUN cargo build --release --target=x86_64-unknown-linux-musl

FROM scratch
WORKDIR /app
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/doh-im-dumb /app/doh-im-dumb
ENTRYPOINT ["/app/doh-im-dumb"]