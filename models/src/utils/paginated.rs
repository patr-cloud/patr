use serde::{Deserialize, Serialize};
use typed_headers::{
	http::{header::ValueIter, HeaderName, HeaderValue},
	Error,
	Header,
	ToValues,
};

use super::{AddTuple, RequiresResponseHeaders};

/// This struct represents a paginated query parameter for the API. It contains
/// the offset and count of the list of items that should be returned. A request
/// that is paginated will always return the total count of items that are
/// available for the query in the `X-Total-Count` header (see the
/// [`TotalCountHeader`] struct for reference).
///
/// ## Example
/// An offset of 10 and a count of 5 would return the items 10, 11, 12, 13 and
/// 14 (assuming the items are zero-indexed). This means that the offset is the
/// index of the first item that should be returned and the count is the number
/// of items that should be returned.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub struct Paginated<T = ()> {
	/// Any other query parameters that should be included in the request.
	pub data: T,
	/// The number of items that should be returned per page.
	pub count: usize,
	/// The page number that should be returned. This is zero-indexed. So to get
	/// the first page, you should set this to 0, and to get the second page,
	/// you should set this to 1, etc.
	pub page: usize,
}

impl<T> Paginated<T> {
	/// The default page size that should be used if no page size is specified.
	/// This is currently set to 25. So if no page size is specified, the API
	/// will return a maximum of 25 items, starting from the first item.
	pub const DEFAULT_PAGE_SIZE: usize = 25;
}

impl<T> Default for Paginated<T>
where
	T: Default,
{
	fn default() -> Self {
		Self {
			data: T::default(),
			count: Self::DEFAULT_PAGE_SIZE,
			page: 0,
		}
	}
}

impl<T> RequiresResponseHeaders for Paginated<T>
where
	T: AddTuple<TotalCountHeader>,
{
	type RequiredResponseHeaders = <T as AddTuple<TotalCountHeader>>::ResultantTuple;
}

/// This struct represents the total count of items that are available for the
/// query. This is used to set the `X-Total-Count` header in the response.
pub struct TotalCountHeader(pub usize);

/// A header that is added to the response to indicate the total number of
/// items that are available for the query (usually for list routes).
static TOTAL_COUNT_HEADER_NAME: HeaderName = HeaderName::from_static("x-total-count");

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
		values.append(HeaderValue::from_str(self.0.to_string().as_str()).unwrap());
	}
}
