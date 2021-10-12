FROM rust:latest as build

RUN rustup target add x86_64-unknown-linux-gnu

WORKDIR /app

COPY . .

RUN cargo build --release --target=x86_64-unknown-linux-gnu

FROM ubuntu:latest

WORKDIR /app

COPY --from=build /app/target/x86_64-unknown-linux-gnu/release/api .

CMD ["init", "--", "/app/api"]
