#![feature(exitcode_exit_method)]

//! A CLI tool for interacting and managing your Patr resources.
//!
//! The binary (`patr`) is a thin wrapper around this library — everything lives
//! here so the integration tests in `tests/` can drive commands directly.

/// All items related to running the CLI goes here
pub mod app;
/// All the commands, arguments and the functionalities for it.
pub mod commands;
/// The errors thrown by the CLI.
pub mod error;
/// Utilities module for helper functions, structs, and enums.
pub mod utils;

/// A prelude that re-exports commonly used items.
pub mod prelude {
	pub use models::prelude::*;
	pub use tracing::{debug, error, info, instrument, trace, warn};

	pub use crate::{
		app::{CommandOutput, OutputType},
		commands::{AppArgs, GlobalArgs, GlobalCommand},
		error::AppError,
		utils::{
			AppState,
			AuthState,
			Channel,
			SearchAndSelect,
			ToJsonValue,
			TtyExpectable,
			WorkspacedArgs,
			clear_screen,
			constants,
			make_request,
		},
	};
}
