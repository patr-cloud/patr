use axum_extra::routing::TypedPath;
use serde::{de::DeserializeOwned, Serialize};

use crate::utils::IntoAxumResponse;

pub trait ApiEndpoint
where
	Self: Sized + Clone + Send + 'static,
	Self::RequestPath: TypedPath
		+ Serialize
		+ DeserializeOwned
		+ Clone
		+ Send
		+ Sync
		+ 'static,
	Self::RequestQuery:
		Serialize + DeserializeOwned + Clone + Send + Sync + 'static,
	Self::RequestBody: 'static,

	Self::ResponseBody: IntoAxumResponse + 'static,
{
	const METHOD: reqwest::Method;
	type RequestPath;
	type RequestQuery;
	type RequestBody;

	type ResponseBody;
}
