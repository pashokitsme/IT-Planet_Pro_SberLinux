FROM rust:slim-bookworm AS builder

WORKDIR /src
COPY . .

RUN cargo update

RUN cargo build --release

FROM scratch
COPY --from=builder /src/target/release/backups /
