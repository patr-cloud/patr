use eve_rs::Error;
use thiserror::Error as ThisError;

#[derive(ThisError, Debug)]
pub enum APIError {
	#[error("Incorrect Parameter")]
	WrongParameters,

	#[error("No Error")]
	None,
}

impl Default for APIError {
	fn default() -> Self {
		return APIError::None;
	}
}

pub trait HasErrorData<Value, TErrorData>
where
	Value: Send + Sync,
	TErrorData: Default + Send + Sync,
{
	fn data(self, data: TErrorData) -> Result<Value, Error<TErrorData>>;
}

impl<Value, TErrorData> HasErrorData<Value, TErrorData>
	for Result<Value, eve_rs::Error<TErrorData>>
where
	Value: Send + Sync,
	TErrorData: Default + Send + Sync,
{
	fn data(self, data: TErrorData) -> Result<Value, Error<TErrorData>> {
		match self {
			Ok(value) => Ok(value),
			Err(err) => Err(err.data(data)),
		}
	}
}
