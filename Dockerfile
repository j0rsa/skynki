ARG BINARY_NAME=app

# --------------- PREPARE BUILDER ENV ------------------------------------------
FROM rust:latest as base
RUN apt update && \
    apt install -y pkg-config libssl-dev && \
    mkdir -p /app/src
WORKDIR /app

FROM base as base-amd64
ARG RUST_TARGET=x86_64-unknown-linux-gnu
# RUN rustup target add $RUST_TARGET

# Inspired by https://github.com/skerkour/black-hat-rust/blob/main/ch_12/rat/docker/Dockerfile.aarch64
FROM base as base-arm64
ARG RUST_TARGET=aarch64-unknown-linux-gnu
RUN apt install -y g++-aarch64-linux-gnu libc6-dev-arm64-cross && \
    rustup target add $RUST_TARGET
ENV CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc \
    CC_aarch64_unknown_linux_gnu=aarch64-linux-gnu-gcc \
    CXX_aarch64_unknown_linux_gnu=aarch64-linux-gnu-g++

FROM base as base-arm32
ARG RUST_TARGET=armv7-unknown-linux-gnueabihf
RUN apt install -y g++-arm-linux-gnueabihf libc6-dev-armhf-cross && \
    rustup target add $RUST_TARGET && \
    rustup toolchain install stable-armv7-unknown-linux-gnueabihf
ENV CARGO_TARGET_ARMV7_UNKNOWN_LINUX_GNUEABIHF_LINKER=arm-linux-gnueabihf-gcc \
    CC_armv7_unknown_linux_gnueabihf=arm-linux-gnueabihf-gcc \
    CXX_armv7_unknown_linux_gnueabihf=arm-linux-gnueabihf-g++
# --------------- END OF PREPARATION ------------------------------------------

# Builder
FROM base-$TARGETARCH AS cargo-build

# Declare args in the builder scope to be able to use it
ARG BINARY_NAME

COPY Cargo.toml Cargo.toml

RUN echo 'fn main() {println!("if you see this, the build broke")}' > src/main.rs && \
    cargo build --release --target=${RUST_TARGET}

# Ubuntu uses dash as default sh by default
SHELL ["/bin/bash", "-c"]

# as binary name but - -> _. Example: cargo-build -> cargo_build
RUN DEP_NAME=${BINARY_NAME//-/_} \
    rm -f target/${RUST_TARGET}/release/deps/${DEP_NAME}*

COPY . .
RUN cargo build --release --target=${RUST_TARGET}


# --------------- PREPARE RUNNER ENV ------------------------------------------
FROM alpine:3.15.4 as runbase
RUN apk add --no-cache ca-certificates && update-ca-certificates
ENV SSL_CERT_FILE=/etc/ssl/certs/ca-certificates.crt
ENV SSL_CERT_DIR=/etc/ssl/certs

FROM runbase as runbase-amd64
ARG RUST_TARGET=x86_64-unknown-linux-musl

FROM runbase as runbase-arm64
ARG RUST_TARGET=aarch64-unknown-linux-gnu

FROM runbase as runbase-arm32
ARG RUST_TARGET=armv7-unknown-linux-gnueabihf
# --------------- END OF PREPARATION ------------------------------------------

#Runner with ssl support
FROM runbase-$TARGETARCH

# Declare args in the runner scope to be able to use it
ARG BINARY_NAME
LABEL authors="red.avtovo@gmail.com"

COPY --from=cargo-build /app/src/target/${RUST_TARGET}/release/${BINARY_NAME} /opt/

ENV RUST_LOG="info"
ENV BINARY_NAME=${BINARY_NAME}

CMD /opt/${BINARY_NAME}