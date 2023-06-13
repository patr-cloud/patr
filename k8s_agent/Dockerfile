FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder 
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release

FROM ubuntu:focal AS runtime
COPY --from=builder /app/target/release/patr_agent /usr/local/bin/patr_agent
RUN apt-get update && apt install -y openssl ca-certificates
ENTRYPOINT ["/usr/local/bin/patr_agent"]
