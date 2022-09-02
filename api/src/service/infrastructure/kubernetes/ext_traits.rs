use std::fmt::Debug;

use async_trait::async_trait;
use either::Either;
use kube::{
	api::DeleteParams,
	client::Status,
	core::ErrorResponse,
	Api,
	Error,
	Result,
};
use serde::de::DeserializeOwned;

#[async_trait]
pub trait DeleteOpt<T> {
	async fn delete_opt(
		&self,
		name: &str,
		dp: &DeleteParams,
	) -> Result<Option<Either<T, Status>>>;
}

#[async_trait]
impl<T> DeleteOpt<T> for Api<T>
where
	T: Clone + DeserializeOwned + Debug,
{
	async fn delete_opt(
		&self,
		name: &str,
		dp: &DeleteParams,
	) -> Result<Option<Either<T, Status>>> {
		match self.delete(name, dp).await {
			Ok(obj) => Ok(Some(obj)),
			Err(Error::Api(ErrorResponse { code: 404, .. })) => Ok(None),
			Err(err) => Err(err),
		}
	}
}
