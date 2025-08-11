use std::{collections::BTreeMap, fmt::Debug};

use headers::{Error, Header};
use http::{HeaderName, HeaderValue};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use super::{AddTuple, RequiresResponseHeaders};
use crate::{prelude::*, rbac::ResourceType};

/// A trait that represents a resource that can be listed. This is used to
/// define the fields that can be used to sort the resource in a paginated
/// request. The `FieldsList` associated type should be a type that can be used
/// to represent the fields that can be used to sort the resource. This is
/// typically an enum that contains the fields that can be used to sort the
/// resource.
pub trait ListableResource {
	/// The type that represents the fields that can be used to sort the
	/// resource in a paginated request. This is typically an enum that contains
	/// the fields that can be used to sort the resource.
	type FieldList: Debug + Clone + Serialize + DeserializeOwned + PartialEq + Eq + PartialOrd + Ord;

	/// The type that represents the search query that can be used to filter the
	/// resource in a paginated request. This is typically a struct that
	/// contains the fields that can be used to filter the resource.
	type SearchStruct: Debug
		+ IsEmpty
		+ Clone
		+ Serialize
		+ DeserializeOwned
		+ PartialEq
		+ Eq
		+ PartialOrd
		+ Ord;
}

impl ListableResource for () {
	type FieldList = ();
	type SearchStruct = ();
}

/// This struct represents a paginated query parameter for the API.
///
/// It contains the offset and count of the list of items that should be
/// returned. A request that is paginated will always return the total count of
/// items that are available for the query in the `X-Total-Count` header (see
/// the [`TotalCountHeader`] struct for reference).
///
/// ## Example
/// An offset of 10 and a count of 5 would return the items 10, 11, 12, 13 and
/// 14 (assuming the items are zero-indexed). This means that the offset is the
/// index of the first item that should be returned and the count is the number
/// of items that should be returned.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub struct ListResourceQuery<R, Q = ()>
where
	R: ListableResource,
{
	/// Sort order of the items.
	#[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
	pub sort: BTreeMap<R::FieldList, SortOrder>,
	/// Search query that can be used to filter items in the list based on the
	/// fields that are available in the resource.
	#[serde(default, skip_serializing_if = "IsEmpty::is_empty")]
	pub search: R::SearchStruct,
	/// The number of items that should be returned per page.
	#[serde(default = "default_page_size")]
	pub count: usize,
	/// The page number that should be returned. This is zero-indexed. So to get
	/// the first page, you should set this to 0, and to get the second page,
	/// you should set this to 1, etc.
	#[serde(default)]
	pub page: usize,
	/// Any other query parameters that should be included in the request.
	#[serde(flatten)]
	pub additional_query: Q,
}

impl ListResourceQuery<()> {
	/// The default page size that should be used if no page size is specified.
	/// This is currently set to 25. So if no page size is specified, the API
	/// will return a maximum of 25 items, starting from the first item.
	pub const DEFAULT_PAGE_SIZE: usize = 25;
}

/// Get the default page size that should be used if no page size is
/// specified. This is currently set to 25. So if no page size is specified,
/// the API will return a maximum of 25 items, starting from the first item.
const fn default_page_size() -> usize {
	ListResourceQuery::DEFAULT_PAGE_SIZE
}

impl<T, Q> Default for ListResourceQuery<T, Q>
where
	T: ListableResource,
	T::SearchStruct: Default,
	Q: Default,
{
	fn default() -> Self {
		Self {
			search: T::SearchStruct::default(),
			sort: BTreeMap::new(),
			additional_query: Q::default(),
			count: ListResourceQuery::DEFAULT_PAGE_SIZE,
			page: 0,
		}
	}
}

impl<T, Q> RequiresResponseHeaders for ListResourceQuery<T, Q>
where
	T: ListableResource,
	Q: AddTuple<TotalCountHeader>,
{
	type RequiredResponseHeaders = <Q as AddTuple<TotalCountHeader>>::ResultantTuple;
}

/// This struct represents a search query for a resource. It contains the
/// resource ID that should be used to search for the resource. This is used
/// to filter the resources that are returned in a paginated request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceSearcher<const R: ResourceType> {
	/// The ID of the resource that should be searched for. This is used to
	/// filter the resources that are returned in a paginated request.
	pub resource_id: Uuid,
}

/// This struct represents the total count of items that are available for the
/// query. This is used to set the `X-Total-Count` header in the response.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord)]
pub struct TotalCountHeader(pub usize);

/// A header that is added to the response to indicate the total number of
/// items that are available for the query (usually for list routes).
static TOTAL_COUNT_HEADER_NAME: HeaderName = HeaderName::from_static("x-total-count");

impl Header for TotalCountHeader {
	fn name() -> &'static HeaderName {
		&TOTAL_COUNT_HEADER_NAME
	}

	fn decode<'i, I>(values: &mut I) -> Result<Self, Error>
	where
		Self: Sized,
		I: Iterator<Item = &'i HeaderValue>,
	{
		let value = values.next().ok_or_else(headers::Error::invalid)?;

		let count = value
			.to_str()
			.map_err(|_| headers::Error::invalid())?
			.parse::<usize>()
			.map_err(|_| headers::Error::invalid())?;

		Ok(Self(count))
	}

	fn encode<E>(&self, values: &mut E)
	where
		E: Extend<HeaderValue>,
	{
		values.extend(std::iter::once(
			HeaderValue::from_str(&self.0.to_string()).expect("HeaderValue should be valid UTF-8"),
		));
	}
}
