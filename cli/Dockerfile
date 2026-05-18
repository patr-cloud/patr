# Build context should be the repo root:

# Generate TS bindings from Rust models
FROM rust:1 AS bindings

WORKDIR /app
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY . .

ENV SQLX_OFFLINE=true
RUN cargo bindings

FROM node:24-alpine as frontend

RUN corepack enable && corepack prepare pnpm@10.33.0 --activate

WORKDIR /app/frontend

COPY frontend/package.json frontend/pnpm-lock.yaml frontend/pnpm-workspace.yaml ./
RUN pnpm install --frozen-lockfile

COPY frontend/ ./
COPY assets/frontend/ /app/assets/frontend/
COPY --from=bindings /app/frontend/src/bindings ./src/bindings

ENV VITE_BUILD_TARGET=csr
RUN pnpm build

FROM rust:1 as build

RUN apt update && apt install -y build-essential clang

ARG PATR_BUILD_CHANNEL
ARG PATR_BUILD_SHA
ENV PATR_BUILD_CHANNEL=${PATR_BUILD_CHANNEL}
ENV PATR_BUILD_SHA=${PATR_BUILD_SHA}

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
