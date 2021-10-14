FROM rust:latest as build

RUN rustup target add x86_64-unknown-linux-gnu

WORKDIR /app

COPY . .

ENV SQLX_OFFLINE=true
RUN cargo build --release --target=x86_64-unknown-linux-gnu

FROM ubuntu:latest

WORKDIR /app

RUN apt update && apt install -y libssl-dev ca-certificates dumb-init
COPY --from=build /app/target/x86_64-unknown-linux-gnu/release/api .
COPY --from=build /app/assets assets/

CMD ["dumb-init", "/app/api"]
