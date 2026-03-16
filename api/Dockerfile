FROM rust:1 as build

WORKDIR /app

COPY . .

ENV SQLX_OFFLINE=true
RUN cargo build --release --package api

FROM rust:1

WORKDIR /app

RUN apt update && apt install -y libssl-dev ca-certificates dumb-init
COPY --from=build /app/target/release/api .

CMD ["dumb-init", "./api"]
