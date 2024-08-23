FROM rust:1 as build

RUN rustup target add x86_64-unknown-linux-gnu

WORKDIR /app

RUN cargo install cargo-leptos

COPY . .

ENV SQLX_OFFLINE=true
RUN cargo leptos build --release

FROM rust:1

WORKDIR /app

RUN apt update && apt install -y libssl-dev ca-certificates dumb-init
COPY --from=build /usr/local/cargo/bin/. /usr/local/cargo/bin/.
COPY --from=build /app/target/release/api .

CMD ["cargo", "leptos", "serve"]
