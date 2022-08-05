pub use api_macros::ErrorResponse;

use eve_rs::{Error as EveError, AsError};

#[derive(ErrorResponse, Debug)]
pub enum APIError {

	#[status = 404]
	#[error = "Incorrect"]
	WrongParameters,

}

trait AsErrorResponse {

	fn get_status_code(&self) -> u16;

	fn get_body(&self) -> &str;

}

pub trait EnumError<V, U, T> {

	fn with_error(self, err: T) -> Result<V, U>;

}

impl<Value, TErrorData, T> EnumError<Value, EveError<TErrorData>, T>
	for Result<Value, EveError<TErrorData>>
where
	T: AsErrorResponse,
	Value: Send + Sync,
	TErrorData: Default + Send + Sync,
{
	
	fn with_error(self, err: T) -> Result<Value, EveError<TErrorData>> {
		self.status(err.get_status_code()).body(err.get_body())
	}

}
