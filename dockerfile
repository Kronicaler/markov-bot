FROM rust:slim-trixie AS builder
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

FROM debian:trixie-slim AS release

RUN apt-get update && apt-get install -y unzip ffmpeg libssl3 libopus-dev curl ca-certificates python3 python3-venv && rm -rf /var/lib/apt/lists/*;

RUN python3 -m venv /opt/yt-dlp && \
    /opt/yt-dlp/bin/python -m pip install --upgrade pip && \
    /opt/yt-dlp/bin/python -m pip install \
        "yt-dlp[default,curl-cffi]" \
        yt-dlp-ejs

ENV PATH="/opt/yt-dlp/bin:$PATH"

RUN curl -L https://deno.land/install.sh -o denoinstall.sh;
RUN chmod a+rx ./denoinstall.sh;

ENV DENO_INSTALL=/usr/local/
RUN ./denoinstall.sh -y;

COPY --link --from=builder /markov_bot /app/markov_bot

WORKDIR /app
ENTRYPOINT ["./markov_bot"]

