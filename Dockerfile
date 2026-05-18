FROM rust:slim AS builder

RUN apt-get update && \
    apt-get install -y --no-install-recommends curl npm libc-dev binaryen

RUN npm install -g sass

RUN curl --proto '=https' --tlsv1.2 -LsSf https://github.com/leptos-rs/cargo-leptos/releases/latest/download/cargo-leptos-installer.sh | sh

# Add the WASM target
RUN rustup target add wasm32-unknown-unknown

WORKDIR /work
COPY . .

RUN cargo leptos build --release -vv

FROM debian:stable-slim AS runner

WORKDIR /app

COPY --from=builder /work/target/release/mod-explorer /app/
COPY --from=builder /work/target/site /app/site
COPY --from=builder /work/Cargo.toml /app/

ENV RUST_LOG="info"
ENV LEPTOS_SITE_ROOT=./site
EXPOSE 3000

ENTRYPOINT ["mod-explorer"]
