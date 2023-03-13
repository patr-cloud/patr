mod eve_context;
mod eve_middlewares;

pub mod billing;
pub mod constants;
pub mod handlebar_registry;
pub mod logger;
pub mod settings;
pub mod validator;

use api_models::ErrorType;
pub use eve_context::*;
pub use eve_middlewares::*;
use eve_rs::Error as EveError;

pub type ErrorData = ErrorType;
pub type Error = EveError<ErrorData>;
