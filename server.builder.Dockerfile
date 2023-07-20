FROM rust:slim
RUN rustup target add x86_64-unknown-linux-musl
