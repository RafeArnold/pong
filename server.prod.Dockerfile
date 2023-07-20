FROM pong-builder AS builder
WORKDIR /build
COPY . .
RUN cargo build --release --bin server --target x86_64-unknown-linux-musl

FROM alpine
COPY --from=builder /build/target/x86_64-unknown-linux-musl/release/server /usr/local/bin/app
CMD ["app"]
