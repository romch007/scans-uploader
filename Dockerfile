FROM rust:1 AS builder

ENV TINI_VERSION=v0.19.0
ADD https://github.com/krallin/tini/releases/download/${TINI_VERSION}/tini-static /tini
RUN chmod +x /tini

RUN cargo install cargo-build-deps

WORKDIR /app

RUN cargo new --bin scans-uploader
WORKDIR /app/scans-uploader

COPY Cargo.toml Cargo.lock ./
RUN cargo build-deps --release

COPY src ./src
RUN cargo build --release
RUN strip target/release/scans-uploader

FROM gcr.io/distroless/cc-debian12:nonroot

WORKDIR /app

COPY --from=builder /app/scans-uploader/target/release/scans-uploader /app
COPY --from=builder /tini /tini

ENTRYPOINT ["/tini" , "--"]

CMD ["/app/scans-uploader"]
