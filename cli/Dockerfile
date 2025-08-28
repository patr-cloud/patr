FROM rust:1 as build

RUN apt update && apt install -y build-essential clang

WORKDIR /app

COPY . .

RUN cargo build --release --package cli

FROM ubuntu:noble

WORKDIR /app

COPY --from=build /app/target/release/patr /usr/local/bin/patr

ENTRYPOINT ["/usr/local/bin/patr"]

CMD ["--help"]
