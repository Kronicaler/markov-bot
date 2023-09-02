FROM rust:1.72 as builder

# Check if we are doing cross-compilation, if so we need to add in some more dependencies and run rustup
RUN apt-get update && apt-get install --no-install-recommends -y cmake;

WORKDIR /app/

COPY . .

ENV SQLX_OFFLINE=true
# Compile or crosscompile
RUN cargo build --release;

FROM debian:bookworm-slim as release

RUN apt-get update && apt-get install --no-install-recommends -y libssl3;
WORKDIR /app
COPY --from=builder /app/target/release/markov_bot /app/markov_bot

ENTRYPOINT ["/app/markov_bot"]
