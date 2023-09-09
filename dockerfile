FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder 
WORKDIR /app

RUN apt-get update && apt-get install --no-install-recommends -y cmake;
ENV SQLX_OFFLINE=true

COPY --from=planner /app/recipe.json recipe.json

RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim as release

RUN apt-get update && apt-get install --no-install-recommends -y libssl3 libopus-dev curl ca-certificates python3;

RUN curl -L https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp -o /usr/local/bin/yt-dlp;
RUN chmod a+rx /usr/local/bin/yt-dlp;

WORKDIR /app
COPY --from=builder /app/target/release/markov_bot /app/markov_bot

ENTRYPOINT ["/app/markov_bot"]
