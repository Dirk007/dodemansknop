FROM rust:1.65-bullseye as builder
WORKDIR /usr/src/dodemansknop

COPY . .
RUN cargo build -r

FROM debian:bullseye-slim
COPY --from=builder /usr/src/dodemansknop/target/release/dodemansknop /usr/local/bin/dodemansknop

ENTRYPOINT ["/usr/local/bin/dodemansknop"]
CMD ["--config", "/etc/dodemansknop/config.yaml"]