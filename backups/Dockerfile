FROM rust:slim-bookworm AS builder

WORKDIR /src
COPY . .

ENV USE_DOCKER=1

RUN cargo update

RUN cargo build --release

FROM debian:bookworm-slim

WORKDIR /bin

ARG RUST_LOG=info
ARG RUST_LOG_STYLE=always

COPY --from=builder /src/target/release/backups backups

CMD ["backups", "start"]
