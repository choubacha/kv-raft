FROM rust:1.26.2

RUN mkdir -p /app/src
WORKDIR app

COPY Cargo.toml .
COPY Cargo.lock .

RUN echo "" > /app/src/lib.rs && cargo check

RUN apt-get -y update && apt-get -y install protobuf-compiler

# Build actual source
ADD . .

RUN cargo build

WORKDIR /app/target/debug
