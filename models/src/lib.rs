#![forbid(unsafe_code)]
#![deny(missing_docs, clippy::missing_docs_in_private_items)]
#![cfg_attr(
	debug_assertions,
	allow(unused_variables, dead_code, unused_mut),
	allow(missing_docs, clippy::missing_docs_in_private_items)
)]

pub mod api;
pub mod ci;
pub mod cloudflare;
pub mod iaac;
pub mod utils;

mod error;

pub use self::error::*;
