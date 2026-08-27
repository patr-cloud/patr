set shell := ["bash", "-uc"]

mod api 'api/tests/Justfile'
mod cli 'cli/tests/Justfile'
mod e2e

# Regenerate the TypeScript bindings: both the per-type files (ts-rs, via the
# `cargo bindings` alias) and the index.ts barrel that re-exports them. ts-rs
# does not manage the barrel, so it has to be rebuilt here or it silently drifts
# (missing exports for new types, dangling exports for removed ones).
bindings:
    #!/usr/bin/env bash
    set -euo pipefail
    # Wipe first. ts-rs only ever writes files, it never deletes them, and the
    # barrel below is built by listing this directory — so a leftover file for
    # a type that no longer exists gets re-exported, and the barrel looks
    # perfectly self-consistent locally while CI (which generates into a clean
    # checkout) sees dangling exports and fails to compile.
    find frontend/src/bindings -maxdepth 1 -name '*.ts' -delete
    cargo bindings
    cd frontend/src/bindings
    {
        echo '// Auto-generated barrel — re-exports every binding file. Regenerate via `just bindings`.'
        echo ''
        ls *.ts | grep -v '^index.ts$' | LC_ALL=C sort | \
            awk '{ sub(/\.ts$/, ""); print "export type * from \"./" $0 "\";" }'
    } > index.ts
    cd ../..
    pnpm exec prettier --write src/bindings/index.ts

prepare:
    cargo sqlx prepare --workspace -- --features cloud --manifest-path api/Cargo.toml
    mkdir -p target/.sqlx-stash
    mv .sqlx/query-*.json target/.sqlx-stash/
    cargo sqlx prepare --workspace -- --no-default-features --manifest-path api/Cargo.toml
    mv target/.sqlx-stash/query-*.json .sqlx/
    rm -rf target/.sqlx-stash
