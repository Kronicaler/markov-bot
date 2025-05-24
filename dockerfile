FROM rust:slim-bookworm AS builder
WORKDIR /app

RUN apt-get update && apt-get install -y cmake pkg-config openssl libssl-dev build-essential wget && rm -rf /var/lib/apt/lists/*;

RUN rustup default nightly
RUN rustup update

ENV SQLX_OFFLINE=true

# Build the project

RUN --mount=type=bind,source=src,target=src \
    --mount=type=bind,source=.sqlx,target=.sqlx \
    --mount=type=bind,source=migrations,target=migrations \
    --mount=type=bind,source=Cargo.toml,target=Cargo.toml \
    --mount=type=bind,source=Cargo.lock,target=Cargo.lock \
    --mount=type=cache,target=/app/target/ \
    --mount=type=cache,target=/usr/local/cargo/git/db \
    --mount=type=cache,target=/usr/local/cargo/registry/ \
cargo build --locked --release && \
cp ./target/release/markov_bot /markov_bot

FROM debian:bookworm-slim AS release

RUN apt-get update && apt-get install -y ffmpeg libssl3 libopus-dev curl ca-certificates python3 && rm -rf /var/lib/apt/lists/*;

RUN curl -L https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp -o /usr/local/bin/yt-dlp;
RUN chmod a+rx /usr/local/bin/yt-dlp;

COPY --link --from=builder /markov_bot /app/markov_bot

WORKDIR /app
ENTRYPOINT ["./markov_bot"]

