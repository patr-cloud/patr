use serde::{Deserialize, Serialize};

/// Ordering of the list for paginated requests
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ListOrder {
	/// Ascending order
	Ascending,
	/// Descending order
	#[default]
	Descending,
}

/// Which field to order the list by for paginated requests
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ListOrderBy {
	/// Order the list by the status of the resource
	Status,
	/// Order the list by the name of the resource
	Name,
	/// Order the list by when the resource was last updated
	LastUpdated,
	/// Order the list by when the resource was created
	#[default]
	Created,
}
