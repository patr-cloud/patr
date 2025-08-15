FROM rust:1 as build

WORKDIR /app

RUN cargo install cargo-leptos

COPY . .

ENV SQLX_OFFLINE=true
RUN cargo leptos build --release --project api

FROM rust:1

WORKDIR /app

RUN apt update && apt install -y libssl-dev ca-certificates dumb-init
ENV LEPTOS_ENV=PROD
ENV LEPTOS_SITE_ROOT=/app
COPY --from=build /app/target/dashboard /app/.
COPY --from=build /app/target/release/api .

CMD ["cargo", "leptos", "serve"]
