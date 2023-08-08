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

#[derive(
	Debug,
	Clone,
	Copy,
	PartialEq,
	Eq,
	Hash,
	PartialOrd,
	Ord,
	Serialize,
	Deserialize,
)]
pub struct LoginId(pub Uuid);

static LOGIN_ID_HEADER_NAME: HeaderName = HeaderName::from_static("x-login-id");

impl Header for LoginId {
	fn name() -> &'static HeaderName {
		&LOGIN_ID_HEADER_NAME
	}

	fn from_values(
		values: &mut ValueIter<'_, HeaderValue>,
	) -> Result<Option<Self>, Error>
	where
		Self: Sized,
	{
		let Some(value) = values.next() else {
			return Ok(None);
		};
		let login_id = Uuid::parse_str(
			value.to_str().map_err(|_| Error::invalid_value())?,
		)
		.map_err(|_| Error::invalid_value())?;
		values.into_iter().for_each(drop);
		Ok(Some(LoginId(login_id)))
	}

	fn to_values(&self, values: &mut ToValues) {
		values.append(
			HeaderValue::from_str(self.0.to_string().as_str()).unwrap(),
		);
	}
}

pub trait HasHeader<H>
where
	H: Header,
{
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
impl_has_headers!(
	H1, H2, H3, H4, H5, H6, H7, H8, H9, H10, H11, H12, H13, H14, H15
);
impl_has_headers!(
	H1, H2, H3, H4, H5, H6, H7, H8, H9, H10, H11, H12, H13, H14, H15, H16
);

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

pub trait Headers: Sized {
	fn to_header_map(&self) -> HeaderMap;
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

pub trait RequiresResponseHeaders {
	type RequiredResponseHeaders;
}

impl RequiresResponseHeaders for () {
	type RequiredResponseHeaders = ();
}

pub trait RequiresRequestHeaders {
	type RequiredRequestHeaders;
}

impl RequiresRequestHeaders for () {
	type RequiredRequestHeaders = ();
}
