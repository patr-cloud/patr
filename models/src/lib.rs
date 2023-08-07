#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::missing_docs_in_private_items)]
#![cfg_attr(
	debug_assertions,
	allow(unused_variables, dead_code, unused_mut),
	allow(missing_docs, clippy::missing_docs_in_private_items)
)]
#![feature(try_trait_v2)]

pub mod api;
pub mod ci;
pub mod cloudflare;
pub mod iaac;
pub mod utils;

pub mod prelude {
	pub use crate::utils::{LoginId, Paginated, TotalCountHeader, Uuid};
}

mod error;

pub use self::error::*;
