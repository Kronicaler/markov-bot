FROM rust:1.72 as builder

RUN apt-get update && apt-get install --no-install-recommends -y cmake;

WORKDIR /app/

COPY . .

ENV SQLX_OFFLINE=true

RUN cargo build --release;

FROM debian:bookworm-slim as release

RUN apt-get update && apt-get install --no-install-recommends -y libssl3 libopus-dev curl ca-certificates python3;

RUN curl -L https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp -o /usr/local/bin/yt-dlp;
RUN chmod a+rx /usr/local/bin/yt-dlp;

WORKDIR /app
COPY --from=builder /app/target/release/markov_bot /app/markov_bot

ENTRYPOINT ["/app/markov_bot"]
