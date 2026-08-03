#![allow(missing_docs, clippy::missing_docs_in_private_items)]

//! Integration tests for the `patr` CLI.
//!
//! The CLI's API base URL is baked in at compile time, so the suite points it
//! at a stub server by building with `PATR_TEST_API_BASE_URL` set — see
//! `cli/tests/Justfile`, or run via `just cli::test`. That also means every
//! test shares one listener, so the suite runs single-threaded.

pub mod setup;

/// Tests for `patr apply`.
pub mod apply;
