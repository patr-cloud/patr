use std::{fmt::Debug, future::Future};

use either::Either;
use kube::{api::DeleteParams, core::Status, error::ErrorResponse, Api, Error};
use serde::de::DeserializeOwned;

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
	fn delete_opt(
		&self,
		name: &str,
		dp: &DeleteParams,
	) -> impl Future<Output = Result<Option<Either<K, Status>>, Error>> {
		async {
			match self.delete(name, dp).await {
				Ok(obj) => Ok(Some(obj)),
				Err(Error::Api(ErrorResponse { code: 404, .. })) => Ok(None),
				Err(err) => Err(err),
			}
		}
	}
}
