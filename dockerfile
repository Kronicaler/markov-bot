FROM rust:slim-bookworm AS builder-1
WORKDIR /app

RUN apt-get update && apt-get install -y cmake pkg-config openssl libssl-dev build-essential wget && rm -rf /var/lib/apt/lists/*;

ENV SCCACHE_VERSION=0.5.0

RUN ARCH= && alpineArch="$(dpkg --print-architecture)" \
      && case "${alpineArch##*-}" in \
        amd64) \
          ARCH='x86_64' \
          ;; \
        arm64) \
          ARCH='aarch64' \
          ;; \
        *) ;; \
      esac \
    && wget -O sccache.tar.gz https://github.com/mozilla/sccache/releases/download/v${SCCACHE_VERSION}/sccache-v${SCCACHE_VERSION}-${ARCH}-unknown-linux-musl.tar.gz \
    && tar xzf sccache.tar.gz \
    && mv sccache-v*/sccache /usr/local/bin/sccache \
    && chmod +x /usr/local/bin/sccache

ENV RUSTC_WRAPPER=/usr/local/bin/sccache

RUN rustup default nightly
RUN rustup update

FROM builder-1 AS builder-2

# Pre-compile dependencies
WORKDIR /build

RUN cargo init --name rust-docker

COPY Cargo.toml Cargo.lock ./

RUN --mount=type=cache,target=/root/.cache cargo fetch && \
    cargo build && \
    cargo build --release && \
    rm src/*.rs

FROM builder-2 AS builder-3

# Build the project
COPY src src
COPY migrations migrations 
COPY .sqlx .sqlx

ENV SQLX_OFFLINE=true
RUN --mount=type=cache,target=/root/.cache touch src/main.rs && \
    cargo build --release

FROM debian:bookworm-slim AS release

RUN apt-get update && apt-get install -y ffmpeg libssl3 libopus-dev curl ca-certificates python3 && rm -rf /var/lib/apt/lists/*;

RUN curl -L https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp -o /usr/local/bin/yt-dlp;
RUN chmod a+rx /usr/local/bin/yt-dlp;

COPY --link --from=builder-3 /build/target/release/markov_bot /app/markov_bot

WORKDIR /app
ENTRYPOINT ["./markov_bot"]

