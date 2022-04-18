# Supported targets:
# - x86_64-unknown-linux-musl
# - armv7-unknown-linux-musleabihf
ARG TARGET=x86_64-unknown-linux-musl
ARG BINARY_NAME=app

# Builder
FROM ekidd/rust-musl-builder:stable AS cargo-build

# Declare args in the builder scope to be able to use it
ARG TARGET
ARG BINARY_NAME

COPY Cargo.toml Cargo.toml

RUN mkdir src/ && \
    echo "fn main() {println!(\"if you see this, the build broke\")}" > src/main.rs && \
    cargo build --release --target=$TARGET

# as binary name but - -> _. Example: cargo-build -> cargo_build
RUN export DEP_NAME=${BINARY_NAME//-/_} &&\
    rm -f target/$TARGET/release/deps/${DEP_NAME}*

COPY . .
RUN cargo build --release --target=$TARGET

#Runner with ssl support
FROM alpine:3.10

# Declare args in the runner scope to be able to use it
ARG TARGET
ARG BINARY_NAME
LABEL authors="red.avtovo@gmail.com"

COPY --from=cargo-build /home/rust/src/target/$TARGET/release/$BINARY_NAME /opt/

ENV RUST_LOG="info"
ENV BINARY_NAME=$BINARY_NAME
RUN apk add --no-cache ca-certificates && update-ca-certificates
ENV SSL_CERT_FILE=/etc/ssl/certs/ca-certificates.crt
ENV SSL_CERT_DIR=/etc/ssl/certs

CMD /opt/${BINARY_NAME}