FROM rust:1.61.0 as builder
WORKDIR /usr/src/patr_agent
COPY . .
RUN cargo install --path .

FROM debian:buster-slim
RUN apt-get update & \
    apt-get install -y extra-runtime-dependencies & \
    rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/patr_agent /usr/local/bin/patr_agent
CMD ["patr_agent"]