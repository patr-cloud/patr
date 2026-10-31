# Build context should be the repo root:

FROM rust:1 as build

RUN apt update && apt install -y build-essential clang

ARG PATR_BUILD_CHANNEL
ARG PATR_BUILD_SHA
# Left as a bare ARG so a build without it falls back to the Cargo.toml version
# instead of baking in an empty string.
ARG PATR_BUILD_VERSION
ENV PATR_BUILD_CHANNEL=${PATR_BUILD_CHANNEL}
ENV PATR_BUILD_SHA=${PATR_BUILD_SHA}

WORKDIR /app

COPY . .

RUN cargo build --release --package cli --locked

FROM ubuntu:noble

WORKDIR /app

RUN apt update && apt install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=build /app/target/release/patr /usr/local/bin/patr

ENTRYPOINT ["/usr/local/bin/patr"]

CMD ["--help"]
