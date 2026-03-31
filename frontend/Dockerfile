# Build context should be the repo root:

# Generate TS bindings from Rust models
FROM rust:1 AS bindings

WORKDIR /app
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY . .

ENV SQLX_OFFLINE=true
RUN cargo bindings

FROM node:22-alpine AS base

RUN corepack enable && corepack prepare pnpm@10.30.1 --activate

# Install dependencies
FROM base AS deps

WORKDIR /app/frontend

COPY frontend/package.json frontend/pnpm-lock.yaml ./
RUN pnpm install --frozen-lockfile

# Build the app
FROM base AS build

WORKDIR /app

# Copy assets needed by the SSR build (publicDir references ../assets/frontend)
COPY assets/frontend ./assets/frontend

WORKDIR /app/frontend

COPY --from=deps /app/frontend/node_modules ./node_modules
COPY frontend/ .
COPY --from=bindings /app/frontend/src/bindings ./src/bindings

ARG VITE_TURNSTILE_SITE_KEY
ENV VITE_TURNSTILE_SITE_KEY=${VITE_TURNSTILE_SITE_KEY}

ARG VITE_BASE_URL
ENV VITE_BASE_URL=${VITE_BASE_URL}

RUN pnpm run build

# Run the app
FROM base AS runner

WORKDIR /app

COPY --from=build /app/frontend/.output ./.output

ENV NODE_ENV=production

EXPOSE 3000

CMD ["node", ".output/server/index.mjs"]
