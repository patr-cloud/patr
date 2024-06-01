use std::{fmt::Debug, future::Future};

use either::Either;
use kube::{api::DeleteParams, core::Status, error::ErrorResponse, Api, Error};
use serde::de::DeserializeOwned;

/// Constants used in the Controller
pub mod constants {
	/// The base URL for the Patr API
	pub const API_BASE_URL: &str = if cfg!(debug_assertions) {
		"https://api.patr.cloud"
	} else {
		"http://localhost:3000"
	};
}

pub trait KubeApiExt<K>
where
	K: Clone + DeserializeOwned + Debug,
{
	fn delete_opt(
		&self,
		name: &str,
		dp: &DeleteParams,
	) -> impl Future<Output = Result<Option<Either<K, Status>>, Error>>;
}

impl<K> KubeApiExt<K> for Api<K>
where
	K: Clone + DeserializeOwned + Debug,
{
	async fn delete_opt(
		&self,
		name: &str,
		dp: &DeleteParams,
	) -> Result<Option<Either<K, Status>>, Error> {
		match self.delete(name, dp).await {
			Ok(obj) => Ok(Some(obj)),
			Err(Error::Api(ErrorResponse { code: 404, .. })) => Ok(None),
			Err(err) => Err(err),
		}
	}
}
