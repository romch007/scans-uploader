FROM rust:1-alpine AS builder

RUN apk add --no-cache musl-dev pkgconf openssl-dev

RUN cargo install cargo-build-deps

WORKDIR /app

ENV RUSTFLAGS='-C target-feature=-crt-static'

RUN cargo new --bin slack-uploader
WORKDIR /app/slack-uploader

COPY Cargo.toml Cargo.lock ./
RUN cargo build-deps --release

COPY src ./src
RUN cargo build --release
RUN strip target/release/slack-uploader

FROM alpine

ARG USER=default

RUN apk add --no-cache tini libgcc
RUN adduser -D $USER

WORKDIR /app

USER $USER

COPY --from=builder /app/slack-uploader/target/release/slack-uploader ./

ENTRYPOINT [ "/sbin/tini", "--" ]

CMD ["/app/slack-uploader"]
