FROM rust:slim-bookworm AS builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src src/
COPY build.rs .
COPY libs libs/
COPY shop-mvp shop-mvp/
COPY shop shop/
COPY portfolio portfolio/
COPY router router/

ENV SQLX_OFFLINE=true
RUN cargo build --release -p shop-mvp

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/shop-mvp .
COPY --from=builder /app/shop-mvp/templates templates/
COPY --from=builder /app/shop-mvp/static static/

ENV PORT=8080
EXPOSE 8080

CMD ["./shop-mvp"]
