set shell := ["bash", "-uc"]

mod api 'api/tests/Justfile'
mod e2e

prepare:
    cargo sqlx prepare --workspace -- --features cloud --manifest-path api/Cargo.toml
    mkdir -p target/.sqlx-stash
    mv .sqlx/query-*.json target/.sqlx-stash/
    cargo sqlx prepare --workspace -- --no-default-features --manifest-path api/Cargo.toml
    mv target/.sqlx-stash/query-*.json .sqlx/
    rm -rf target/.sqlx-stash
