#[cfg(not(target_arch = "wasm32"))]
use std::{
	any::Any,
	collections::HashMap,
	sync::{OnceLock, RwLock},
};

#[cfg(not(target_arch = "wasm32"))]
use matchit::Router;
use models::prelude::*;
#[cfg(not(target_arch = "wasm32"))]
use serde::{de::DeserializeOwned, Serialize};

pub use self::{make_request::make_request, stream_request::stream_request};

mod make_request;
mod stream_request;

#[cfg(not(target_arch = "wasm32"))]
/// The type used for the [`API_CALL_REGISTRY`] static. This is a map of all the
/// API calls that are registered to the backend. This is used internally and
/// should not be used by any other part of the code.
type ApiCallRegistryData =
	OnceLock<RwLock<HashMap<http::Method, Router<Box<dyn Any + Send + Sync>>>>>;

#[cfg(not(target_arch = "wasm32"))]
#[doc(hidden)]
/// Used internally for registering API calls to the backend. DO NOT USE THIS ON
/// YOUR OWN. Use the [`make_request`] fn instead.
pub static API_CALL_REGISTRY: ApiCallRegistryData = OnceLock::new();

/// Register an API call to the backend. This will register the API call to the
/// backend so that it can be used by the frontend. This is used internally and
/// should not be used by any other part of the code.
#[cfg(not(target_arch = "wasm32"))]
pub fn register_api_call<E>()
where
	E: ApiEndpoint,
	E::RequestBody: Serialize + DeserializeOwned,
	E::ResponseBody: Serialize + DeserializeOwned,
{
	make_request::register_api_call::<E>();
}
