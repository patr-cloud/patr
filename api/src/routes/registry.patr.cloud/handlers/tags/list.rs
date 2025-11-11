//! GET tags list endpoint handler.
//!
//! This handler retrieves all tags for a repository with pagination support.
//! It returns a JSON response with the repository name and an array of tag
//! names.

use headers::ContentType;

use crate::routes::registry_patr_cloud::prelude::*;

macros::declare_registry_endpoint!(
	/// GET tags list endpoint.
	///
	/// Retrieves all tags for a repository with pagination support.
	ListTags,
	GET "/v2/{workspace_id}/{name}/tags/list" {
		/// The workspace ID
		pub workspace_id: Uuid,
		/// The repository name
		pub name: String,
	},
	auth = true,
	request_headers = {
		/// The authorization header
		pub authorization: BearerToken,
	},
	query = {
		/// Maximum number of tags to return (default: 100)
		#[serde(rename = "n")]
		pub number: Option<i64>,
		/// Last tag name from previous page (for pagination)
		pub last: Option<String>,
	},
	response_headers = {
		/// Content-Type header for JSON response
		pub content_type: ContentType,
		/// Link header for pagination
		pub link: OptionalHeader<Link>,
	}
);

/// Handler for GET /v2/{name}/tags/list
///
/// This handler:
/// 1. Parses and validates the repository name
/// 2. Verifies workspace access
/// 3. Queries the database for tags with pagination
/// 4. Returns JSON response with repository name and tags array
/// 5. Includes Link header if more results are available
///
/// # Requirements
/// - 7.2: List tags for a repository
/// - 13.1: Use typed paths
/// - 13.2: Use typed queries
/// - 13.5: Provide Link headers for pagination
/// - 12.1: Use database transaction
#[instrument(skip(database, user_data))]
pub async fn list_tags(
	AuthenticatedRegistryAppRequest {
		request:
			RegistryProcessedApiRequest {
				path: ListTagsPath {
					workspace_id,
					name: repo_name,
				},
				query: ListTagsQuery { number, last },
				headers: ListTagsRequestHeaders { authorization: _ },
				body: _,
			},
		database,
		redis: _,
		s3: _,
		client_ip: _,
		user_data,
		config: _,
	}: AuthenticatedRegistryAppRequest<'_, ListTagsPath>,
) -> Result<RegistryResponse<ListTagsPath>, RegistryError> {
	info!(
		repository = %repo_name,
		n = ?number,
		last = ?last,
		user_id = %user_data.id,
		"GET tags list request"
	);

	// 2. Verify workspace access
	verify_workspace_access(&user_data, workspace_id)?;
	debug!("Workspace access verified");

	// 3. Determine pagination parameters
	let limit = number.unwrap_or(100).min(1000); // Cap at 1000
	let last_tag = last.as_deref();

	debug!(
		limit = limit,
		last_tag = ?last_tag,
		"Pagination parameters"
	);

	// 4. Query database for tags
	let tags = query!(
		r#"
		SELECT
			tags.name
		FROM 
			container_registry_tag AS tags
		INNER JOIN
			container_registry_repository AS repo
		ON
			tags.repository_id = repo.id
		WHERE
			repo.workspace_id = $1 AND
			repo.name = $2 AND
			(
				$3 = NULL OR tags.name > $3
			)
		ORDER BY
			tags.name ASC
		LIMIT $4;
        "#,
		workspace_id as _,
		&repo_name,
		last_tag,
		limit + 1 // Fetch one extra to check for more results
	)
	.fetch_all(&mut *database)
	.await?
	.into_iter()
	.map(|row| row.name)
	.collect::<Vec<_>>();

	let last_tag = tags.get(limit as usize).cloned();

	let body = TagListBuilder::default()
		.name(repo_name)
		.tags(tags.into_iter().take(limit as usize).collect())
		.build()?;

	debug!(tags_count = tags.len(), "Retrieved tags from database");

	// 6. Build Link header if there are more results
	let link_header = if let Some(last_tag) = last_tag {
		let link_url = format!(
			"/v2/{}/{}/tags/list?n={}&last={}",
			workspace_id, repo_name, limit, last_tag
		);
		debug!(link_url = %link_url, "More results available, adding Link header");
		Some(Link::new(format!("<{}>; rel=\"next\"", link_url)))
	} else {
		debug!("No more results available");
		None
	};

	RegistryResponse::builder()
		.status_code(StatusCode::OK)
		.headers(ListTagsResponseHeaders {
			content_type: ContentType::json(),
			link: OptionalHeader(link_header),
		})
		.body(Body::new(serde_json::to_vec(body)?))
		.build()
		.into_result()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_link_header() {
		let link = Link::new("</v2/repo/tags/list?n=10&last=v1.0>; rel=\"next\"");
		assert_eq!(link.0, "</v2/repo/tags/list?n=10&last=v1.0>; rel=\"next\"");
	}

	#[test]
	fn test_tags_list_response_serialization() {
		let response = TagListBuilder::default()
			.name("workspace-id/my-repo".to_string())
			.tags(vec![
				"v1.0".to_string(),
				"v1.1".to_string(),
				"latest".to_string(),
			])
			.build()
			.unwrap();

		let json = serde_json::to_string(&response).unwrap();
		assert!(json.contains("\"name\""));
		assert!(json.contains("\"tags\""));
		assert!(json.contains("v1.0"));
	}
}
