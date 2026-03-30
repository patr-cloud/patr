use std::fmt::Debug;

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
	type SearchStruct: Debug + IsEmpty + Clone + Serialize + DeserializeOwned + PartialEq;
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ListResourceQuery<R, Q = ()>
where
	R: ListableResource,
{
	/// Sort order of the items.
	#[serde(flatten, default = "None", skip_serializing_if = "Option::is_none")]
	pub sort: Option<SortDetails<R>>,
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
			sort: None,
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

/// This struct represents the sorting details for a resource. It contains the
/// field that should be used to sort the resource and the order in which the
/// resource should be sorted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SortDetails<R>
where
	R: ListableResource,
{
	/// The field that should be used to sort the resource. This is typically
	/// an enum that contains the fields that can be used to sort the resource.
	pub sort_by: R::FieldList,
	/// The order in which the resource should be sorted. This can be either
	/// ascending or descending.
	pub sort_order: SortOrder,
}

/// This struct represents a search query for a resource. It contains the
/// resource ID that should be used to search for the resource. This is used
/// to filter the resources that are returned in a paginated request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ResourceSearcher<const R: ResourceType> {
	/// The ID of the resource that should be searched for. This is used to
	/// filter the resources that are returned in a paginated request.
	pub resource_id: Uuid,
}

impl<const R: ResourceType> Serialize for ResourceSearcher<R> {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		self.resource_id.serialize(serializer)
	}
}

impl<'de, const R: ResourceType> Deserialize<'de> for ResourceSearcher<R> {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		let resource_id = Uuid::deserialize(deserializer)?;
		Ok(ResourceSearcher { resource_id })
	}
}

// For backend

#[cfg(not(target_arch = "wasm32"))]
use sqlx::{encode::IsNull, error::BoxDynError, prelude::*};

#[cfg(not(target_arch = "wasm32"))]
impl<const R: ResourceType> Type<sqlx::Sqlite> for ResourceSearcher<R> {
	fn type_info() -> <sqlx::Sqlite as sqlx::Database>::TypeInfo {
		<Uuid as Type<sqlx::Sqlite>>::type_info()
	}
}

#[cfg(not(target_arch = "wasm32"))]
impl<const R: ResourceType> Type<sqlx::Postgres> for ResourceSearcher<R> {
	fn type_info() -> <sqlx::Postgres as sqlx::Database>::TypeInfo {
		<Uuid as Type<sqlx::Postgres>>::type_info()
	}
}

#[cfg(not(target_arch = "wasm32"))]
impl<'a, const R: ResourceType> Encode<'a, sqlx::Sqlite> for ResourceSearcher<R>
where
	Uuid: Encode<'a, sqlx::Sqlite>,
{
	fn encode_by_ref(
		&self,
		buf: &mut <sqlx::Sqlite as sqlx::Database>::ArgumentBuffer<'a>,
	) -> Result<IsNull, BoxDynError> {
		self.resource_id.encode_by_ref(buf)
	}
}

#[cfg(not(target_arch = "wasm32"))]
impl<'a, const R: ResourceType> Encode<'a, sqlx::Postgres> for ResourceSearcher<R>
where
	Uuid: Encode<'a, sqlx::Postgres>,
{
	fn encode_by_ref(
		&self,
		buf: &mut <sqlx::Postgres as sqlx::Database>::ArgumentBuffer<'a>,
	) -> Result<IsNull, BoxDynError> {
		self.resource_id.encode_by_ref(buf)
	}
}

#[cfg(not(target_arch = "wasm32"))]
impl<'a, const R: ResourceType> Decode<'a, sqlx::Sqlite> for ResourceSearcher<R>
where
	Uuid: Decode<'a, sqlx::Sqlite>,
{
	fn decode(value: <sqlx::Sqlite as sqlx::Database>::ValueRef<'a>) -> Result<Self, BoxDynError> {
		Uuid::decode(value).map(|resource_id| Self { resource_id })
	}
}

#[cfg(not(target_arch = "wasm32"))]
impl<'a, const R: ResourceType> Decode<'a, sqlx::Postgres> for ResourceSearcher<R>
where
	Uuid: Decode<'a, sqlx::Postgres>,
{
	fn decode(
		value: <sqlx::Postgres as sqlx::Database>::ValueRef<'a>,
	) -> Result<Self, BoxDynError> {
		Uuid::decode(value).map(|resource_id| Self { resource_id })
	}
}
