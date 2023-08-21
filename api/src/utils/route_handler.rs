use std::{error::Error as StdError, future::Future, pin::Pin};

use axum::{
	body::HttpBody,
	extract::{FromRequest, FromRequestParts},
	http::Request,
	response::{IntoResponse, Response},
};
use models::{
	utils::{ApiRequest, IntoAxumResponse},
	ApiEndpoint,
	ErrorType,
};

use crate::utils::LastElementIs;

pub trait EndpointHandler<Params, S, B, E>
where
	Params: LastElementIs<ApiRequest<E>>,
	E: ApiEndpoint,
{
	type Future: Future<Output = Response> + Send + 'static;

	fn call(self, req: Request<B>, state: S) -> Self::Future;
}

macro_rules! impl_dto_handler {
	( $($ty:ident),* $(,)? ) => {
	impl<F, Fut, R, S, $($ty,)* E, B>
		EndpointHandler<($($ty,)* ApiRequest<E>,), S, B, E> for F
	where
		F: FnOnce($($ty,)* ApiRequest<E>) -> Fut
			+ Clone
			+ Send
			+ 'static,
		Fut: Future<Output = R>
			+ Send
			+ 'static,
		R: IntoAxumResponse,
		B: HttpBody + Send + 'static,
		<B as HttpBody>::Data: Send,
		<B as HttpBody>::Error: StdError + Send + Sync,
		S: Send + Sync + 'static,
		$($ty: FromRequestParts<S> + Send,)*
		E: ApiEndpoint + Send,
		ApiRequest<E>: FromRequest<S, B>,
		<ApiRequest<E> as FromRequest<S, B>>::Rejection:
			Into<ErrorType> + Send,
	{
		type Future = Pin<
			Box<
				dyn Future<Output = Response> + Send,
			>,
		>;

		#[allow(non_snake_case, unused_mut)]
		fn call(self, req: Request<B>, state: S) -> Self::Future {
			Box::pin(async move {
				let (mut parts, body) = req.into_parts();
				let state = &state;

				$(
					let $ty = match $ty::from_request_parts(&mut parts, state).await {
						Ok(value) => value,
						Err(rejection) => return rejection.into_response(),
					};
				)*

				let req = Request::from_parts(parts, body);

				let result = ApiRequest::<E>::from_request(req, state).await;
				let dto = match result {
					Ok(dto) => dto,
					Err(err) => return Err::<(), _>(err).into_response(),
				};

				self($($ty,)* dto).await.into_response()
			})
		}
	}
	};
}

impl_dto_handler!();
impl_dto_handler!(T1);
impl_dto_handler!(T1, T2);
impl_dto_handler!(T1, T2, T3);
impl_dto_handler!(T1, T2, T3, T4);
impl_dto_handler!(T1, T2, T3, T4, T5);
impl_dto_handler!(T1, T2, T3, T4, T5, T6);
impl_dto_handler!(T1, T2, T3, T4, T5, T6, T7);
impl_dto_handler!(T1, T2, T3, T4, T5, T6, T7, T8);
impl_dto_handler!(T1, T2, T3, T4, T5, T6, T7, T8, T9);
impl_dto_handler!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10);
impl_dto_handler!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11);
impl_dto_handler!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12);
impl_dto_handler!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13);
impl_dto_handler!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14);
impl_dto_handler!(
	T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15
);
