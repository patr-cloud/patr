use axum::http::StatusCode;
use models::{api::workspace::infrastructure::managed_url::*, prelude::*};

use crate::prelude::*;

pub async fn list_managed_url(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: ListManagedURLPath { workspace_id },
				query:
					Paginated {
						data:
							ListManagedURLQuery {
								order,
								order_by,
								filter,
							},
						count,
						page,
					},
				headers:
					ListManagedURLRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: ListManagedURLRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		config: _,
		user_data,
	}: AuthenticatedAppRequest<'_, ListManagedURLRequest>,
) -> Result<AppResponse<ListManagedURLRequest>, ErrorType> {
	info!("Listing ManagedURLs in workspace `{}`", workspace_id);

	AppResponse::builder()
		.body(ListManagedURLResponse { urls: todo!() })
		.headers(ListManagedURLResponseHeaders {
			total_count: todo!(),
		})
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
