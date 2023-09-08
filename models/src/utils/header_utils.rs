use serde::{Deserialize, Serialize};
use typed_headers::{
	http::{header::ValueIter, HeaderMap, HeaderName, HeaderValue},
	Accept,
	AcceptEncoding,
	Allow,
	AuthScheme,
	Authorization,
	ContentCoding,
	ContentEncoding,
	ContentLength,
	ContentType,
	Credentials,
	Error,
	Header,
	Host,
	HttpDate,
	ProxyAuthorization,
	Quality,
	QualityItem,
	ToValues,
	Token68,
};

use super::Uuid;

/// This struct represents a bearer token. It is used to authenticate a user's
/// request to the API. It is used as a header in requests to the API.
///
/// This is a wrapper around [`typed_headers::Token68`].
/// Example: Authorization: Bearer <token>
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BearerToken(pub Token68);

impl Header for BearerToken {
	fn name() -> &'static HeaderName {
		&Authorization::name()
	}

	fn from_values<'a>(
		values: &mut reqwest::header::ValueIter<'a, HeaderValue>,
	) -> Result<Option<Self>, Error>
	where
		Self: Sized,
	{
		Authorization::from_values(values).map(|token| {
			token.and_then(|Authorization(creds)| creds.as_bearer().cloned().map(BearerToken))
		})
	}

	fn to_values(&self, values: &mut ToValues) {
		Authorization::to_values(&Authorization(Credentials::bearer(self.0.clone())), values)
	}
}

/// This struct represents a login ID. It is used to identify a user's login in
/// the database. A user's login can be any way they access the API - Either
/// through the website, through an API request, the CLI or from an OAuth
/// application. It is used as a header in requests to the API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LoginId(pub Uuid);

static LOGIN_ID_HEADER_NAME: HeaderName = HeaderName::from_static("x-login-id");

impl Header for LoginId {
	fn name() -> &'static HeaderName {
		&LOGIN_ID_HEADER_NAME
	}

	fn from_values(values: &mut ValueIter<'_, HeaderValue>) -> Result<Option<Self>, Error>
	where
		Self: Sized,
	{
		let Some(value) = values.next() else {
			return Ok(None);
		};
		let login_id = Uuid::parse_str(value.to_str().map_err(|_| Error::invalid_value())?)
			.map_err(|_| Error::invalid_value())?;
		values.into_iter().for_each(drop);
		Ok(Some(LoginId(login_id)))
	}

	fn to_values(&self, values: &mut ToValues) {
		values.append(HeaderValue::from_str(self.0.to_string().as_str()).unwrap());
	}
}

/// This struct is implemented for all types that can be used as a header in a
/// request to the API. This struct is used in conjunction with the
/// [`HasHeaders`] trait to ensure that a request headers struct has all the
/// required headers that are needed for the query, body, etc.
///
/// This should be implemented for any struct that defines a header. It is
/// already implemented for all types that implement the [`Header`] trait
/// in the [`typed_headers`] crate.
pub trait HasHeader<H>
where
	H: Header,
{
	/// A helper function that returns a reference to the header. Not really
	/// used much in the codebase yet, but kept just for safe measure.
	fn get_header(&self) -> &H;
}

impl<H> HasHeader<H> for H
where
	H: Header,
{
	fn get_header(&self) -> &H {
		self
	}
}

/// This trait is implemented with tuples of elements as a generic (up to 16
/// elements) for any struct that has those headers. It is used to ensure that a
/// request headers struct has all the required headers that are needed for the
/// query, body, etc.
///
/// For example, given a struct `Foo` that has the headers `A` and `B`,
/// `HasHeaders<(A, B)>` is automatically implemented for `Foo` IF AND ONLY IF
/// `Foo` implements `HasHeader<A>` and `HasHeader<B>`.
///
/// More realistic example:
///
/// Given a struct `RequestHeaders` like so:
/// ```rust
/// pub struct RequestHeaders {
/// 	pub accept: Accept,
/// 	pub content_type: ContentType,
/// }
/// ```
///
/// If `RequestHeaders` implements `HasHeader<Accept>` and
/// `HasHeader<ContentType>` like so:
///
/// ```rust
/// impl HasHeader<Accept> for RequestHeaders {
/// 	fn get_header(&self) -> &Accept {
/// 		&self.accept
/// 	}
/// }
///
/// impl HasHeader<ContentType> for RequestHeaders {
/// 	fn get_header(&self) -> &ContentType {
/// 		&self.content_type
/// 	}
/// }
/// ```
///
/// Then `HasHeaders<(Accept, ContentType)>` is automatically implemented for
/// `RequestHeaders`.
///
/// Now, it is indeed cumbersome to implement [`HasHeader`] for every header in
/// a struct. So, the [`macros::HasHeaders`] derive macro can be used to
/// automatically implement `HasHeader` for all headers in a given struct.
///
/// In the above example, the following code is equivalent to the code above:
/// ```rust
/// #[derive(HasHeaders)]
/// pub struct RequestHeaders {
/// 	pub accept: Accept,
/// 	pub content_type: ContentType,
/// }
/// ```
///
/// Now, we can make a function that requires a RequestHeader to have these two
/// headers necessarilly by using the [`HasHeaders`] trait:
/// ```rust
/// // A function that requires the `Accept` and `Content-Type` headers
/// fn foo<T: HasHeaders<(Accept, ContentType)>>(headers: &T) {
/// 	// ...
/// 	let accept: &Accept = headers.get_header();
/// 	let content_type: &ContentType = headers.get_header();
/// }
/// ```
///
/// The best part is that in the above example, even if `T` has more headers, it
/// will still work. This way, we can make functions that require a certain set
/// of headers, and the user can pass in any struct that has those headers, even
/// if it has more headers.
pub trait HasHeaders<T> {}

macro_rules! impl_has_headers {
	() => {
		impl<S> HasHeaders<()> for S {}
	};
	( $($headers:ident),+ $(,)? ) => {
		impl<$($headers,)* S> HasHeaders<($($headers,)*)> for S
		where
			$($headers: Header,)*
			S: $(HasHeader<$headers> +)*
		{
		}
	};
}

impl_has_headers!();
impl_has_headers!(H1);
impl_has_headers!(H1, H2);
impl_has_headers!(H1, H2, H3);
impl_has_headers!(H1, H2, H3, H4);
impl_has_headers!(H1, H2, H3, H4, H5);
impl_has_headers!(H1, H2, H3, H4, H5, H6);
impl_has_headers!(H1, H2, H3, H4, H5, H6, H7);
impl_has_headers!(H1, H2, H3, H4, H5, H6, H7, H8);
impl_has_headers!(H1, H2, H3, H4, H5, H6, H7, H8, H9);
impl_has_headers!(H1, H2, H3, H4, H5, H6, H7, H8, H9, H10);
impl_has_headers!(H1, H2, H3, H4, H5, H6, H7, H8, H9, H10, H11);
impl_has_headers!(H1, H2, H3, H4, H5, H6, H7, H8, H9, H10, H11, H12);
impl_has_headers!(H1, H2, H3, H4, H5, H6, H7, H8, H9, H10, H11, H12, H13);
impl_has_headers!(H1, H2, H3, H4, H5, H6, H7, H8, H9, H10, H11, H12, H13, H14);
impl_has_headers!(H1, H2, H3, H4, H5, H6, H7, H8, H9, H10, H11, H12, H13, H14, H15);
impl_has_headers!(H1, H2, H3, H4, H5, H6, H7, H8, H9, H10, H11, H12, H13, H14, H15, H16);

macro_rules! impl_has_headers_for_standard_header {
	[$($header:ident),+ $(,)?] => {
		$(impl HasHeaders<$header> for $header {})+
	};
}

impl_has_headers_for_standard_header![
	Accept,
	AcceptEncoding,
	Allow,
	AuthScheme,
	Authorization,
	ContentCoding,
	ContentEncoding,
	ContentLength,
	ContentType,
	Credentials,
	Host,
	HttpDate,
	ProxyAuthorization,
	Quality,
	Token68
];

impl<T> HasHeaders<QualityItem<T>> for QualityItem<T> {}
impl<'a> HasHeaders<ToValues<'a>> for ToValues<'a> {}

/// This trait is used to convert a struct to and from a [`HeaderMap`]. This is
/// mostly used for internal purposes and you shouldn't have to implement this
/// by hand. This is automatically implemented for any endpoint defined using
/// the [`macros::declare_api_endpoint`] macro.
pub trait Headers: Sized {
	/// Convert the struct to a [`HeaderMap`].
	fn to_header_map(&self) -> HeaderMap;
	/// Convert the struct from a [`HeaderMap`], returning a [`None`] if the
	/// conversion fails.
	fn from_header_map(map: &HeaderMap) -> Option<Self>;
}

impl Headers for () {
	fn to_header_map(&self) -> HeaderMap {
		HeaderMap::new()
	}

	fn from_header_map(_: &HeaderMap) -> Option<Self> {
		Some(())
	}
}

/// This trait represents the response headers that are required for a certain
/// endpoint. It is used to ensure that a response headers struct has all the
/// required headers that are needed.
///
/// The response headers required should be mentioned as a tuple of headers so
/// that it can be used by the [`HasHeaders`] trait.
pub trait RequiresResponseHeaders {
	/// The response headers that are required for this struct to be a part of
	/// an endpoint. This should be a tuple of headers.
	type RequiredResponseHeaders;
}

impl RequiresResponseHeaders for () {
	type RequiredResponseHeaders = ();
}

/// This trait represents the request headers that are required for a certain
/// endpoint. It is used to ensure that a request headers struct has all the
/// required headers that are needed.
///
/// The request headers required should be mentioned as a tuple of headers so
/// that it can be used by the [`HasHeaders`] trait.
pub trait RequiresRequestHeaders {
	/// The request headers that are required for this struct to be a part of an
	/// endpoint. This should be a tuple of headers.
	type RequiredRequestHeaders;
}

impl RequiresRequestHeaders for () {
	type RequiredRequestHeaders = ();
}
