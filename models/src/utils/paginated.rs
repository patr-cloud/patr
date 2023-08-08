use serde::{Deserialize, Serialize};
use typed_headers::{
	http::{header::ValueIter, HeaderName, HeaderValue},
	Error,
	Header,
	ToValues,
};

use super::{AddTuple, RequiresResponseHeaders};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub struct Paginated<T = ()> {
	pub data: T,
	pub count: usize,
	pub offset: usize,
}

impl<T> RequiresResponseHeaders for Paginated<T>
where
	T: AddTuple<TotalCountHeader>,
{
	type RequiredResponseHeaders =
		<T as AddTuple<TotalCountHeader>>::ResultantTuple;
}

pub struct TotalCountHeader(pub usize);

static TOTAL_COUNT_HEADER_NAME: HeaderName =
	HeaderName::from_static("x-total-count");

impl Header for TotalCountHeader {
	fn name() -> &'static HeaderName {
		&TOTAL_COUNT_HEADER_NAME
	}

	fn from_values(
		values: &mut ValueIter<'_, HeaderValue>,
	) -> Result<Option<Self>, typed_headers::Error>
	where
		Self: Sized,
	{
		let Some(value) = values.next() else {
			return Ok(None);
		};
		let count = value
			.to_str()
			.map_err(|_| Error::invalid_value())?
			.parse::<usize>()
			.map_err(|_| Error::invalid_value())?;
		values.into_iter().for_each(drop);
		Ok(Some(TotalCountHeader(count)))
	}

	fn to_values(&self, values: &mut ToValues) {
		values.append(
			HeaderValue::from_str(self.0.to_string().as_str()).unwrap(),
		);
	}
}
