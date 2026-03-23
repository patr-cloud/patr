FROM node:24-alpine as frontend

RUN corepack enable && corepack prepare pnpm@10.30.1 --activate

WORKDIR /app/frontend

COPY frontend/package.json frontend/pnpm-lock.yaml ./
RUN pnpm install --frozen-lockfile

COPY frontend/ ./
COPY assets/frontend/ /app/assets/frontend/

ENV VITE_BUILD_TARGET=csr
RUN pnpm build

FROM rust:1 as build

RUN apt update && apt install -y build-essential clang

WORKDIR /app

COPY . .
COPY --from=frontend /app/frontend/.output/public /app/frontend/.output/public

RUN cargo build --release --package cli --locked

FROM ubuntu:noble

WORKDIR /app

RUN apt update && apt install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=build /app/target/release/patr /usr/local/bin/patr

ENTRYPOINT ["/usr/local/bin/patr"]

CMD ["--help"]
