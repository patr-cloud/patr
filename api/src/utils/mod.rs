mod eve_context;
mod eve_middlewares;

pub mod billing;
pub mod constants;
pub mod handlebar_registry;
pub mod logger;
pub mod settings;
pub mod validator;

// pub use eve_context::*;
// pub use eve_middlewares::*;
// use eve_rs::Error as EveError;

pub type ErrorData = ();
pub type Error = anyhow::Error;
