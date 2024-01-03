FROM --platform=$BUILDPLATFORM rust:bookworm AS builder
ARG TARGETARCH


# RUN case "$TARGETARCH" in \
#       "arm64") echo armv7-unknown-linux-musleabihf > /rust_target.txt ;; \
#       "amd64") echo x86_64-unknown-linux-musl > /rust_target.txt ;; \
#       *) echo "unknown target: $TARGETARCH" ; exit 1 ;; \
#     esac
RUN case "$TARGETARCH" in \
      "arm64") echo aarch64-unknown-linux-gnu > /rust_target.txt ;; \
      "amd64") echo x86_64-unknown-linux-gnu > /rust_target.txt ;; \
      *) echo "unknown target: $TARGETARCH" ; exit 1 ;; \
    esac


RUN rustup target add $(cat /rust_target.txt)

RUN cargo install cargo-strip

RUN apt update && apt install -y gcc gcc-aarch64-linux-gnu && rm -rf /var/lib/apt/lists/*

RUN case "$TARGETARCH" in \
      "arm64") echo aarch64-linux-gnu-gcc > /rust_linker.txt ;; \
      "amd64") echo gcc > /rust_linker.txt ;; \
      *) echo "unknown target: $TARGETARCH" ; exit 1 ;; \
    esac


# WORKDIR /
# RUN wget https://musl.cc/arm-linux-musleabihf-cross.tgz 
# RUN tar xzvf arm-linux-musleabihf-cross.tgz
# RUN cp -r arm-linux-musleabihf-cross/* /usr && rm -fr arm-linux-musleabihf-cross*


COPY . /app

WORKDIR /app
RUN --mount=type=cache,target=/usr/local/cargo/git,id=${TARGETARCH} \
    --mount=type=cache,target=/usr/local/cargo/registry,id=${TARGETARCH} \
    --mount=type=cache,target=./target,id=${TARGETARCH} \
    echo "Current compilation cache size:" && \
    du -csh ./target /usr/local/cargo/registry /usr/local/cargo/git && \
    ls ./target/ -lha && sleep 1 && \
    export RUSTFLAGS="$RUSTFLAGS  -C linker=$(cat /rust_linker.txt)" && \
    cargo build --release --target $(cat /rust_target.txt) && \
    # cargo strip --target $(cat /rust_target.txt) && \
    # Copy executable out of the cache so it is available in the final image.
    cp target/$(cat /rust_target.txt)/release/doh-im-dumb /exec


FROM --platform=$TARGETPLATFORM debian:bookworm-slim
WORKDIR /app
COPY --from=builder /exec /app/exec
COPY secrets ./secrets
ENTRYPOINT ["/app/exec"]