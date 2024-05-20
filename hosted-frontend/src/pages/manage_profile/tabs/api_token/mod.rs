use leptos_use::{use_cookie, utils::FromToStringCodec};
use models::api::user::ListApiTokensResponse;

use crate::prelude::*;

#[server(LoadApiTokenFn, endpoint = "/user/api-token")]
pub async fn load_api_tokens_list(
	access_token: Option<String>,
) -> Result<Result<ListApiTokensResponse, ErrorType>, ServerFnError> {
	use std::str::FromStr;

	use models::api::user::{ListApiTokensPath, ListApiTokensRequest, ListApiTokensRequestHeaders};

	let api_response = make_api_call::<ListApiTokensRequest>(
		ApiRequest::builder()
			.path(ListApiTokensPath)
			.query(Default::default())
			.headers(ListApiTokensRequestHeaders {
				authorization: BearerToken::from_str(
					format!("Bearer {}", access_token.unwrap_or_default()).as_str(),
				)?,
				user_agent: UserAgent::from_static("hyper/0.12.2"),
			})
			.body(ListApiTokensRequest)
			.build(),
	)
	.await;

	Ok(api_response.map(|res| res.body))
}

#[component]
pub fn ApiTokensTab() -> impl IntoView {
	let data = create_rw_signal(vec![UserApiToken {
		name: "test-token".to_string(),
		expiry: "No Expiry".to_string(),
		created: "3 Days Ago".to_string(),
	}]);

	let (access_token, _) = use_cookie::<String, FromToStringCodec>("access_token");
	let access_token_signal = move || access_token.get();
	let token_list = create_resource(access_token_signal, move |value| async move {
		load_api_tokens_list(value).await
	});

	logging::log!("{:#?}", token_list.get());

	view! {
		<div class="fc-fs-fs full-width full-height px-md py-xl gap-md">
			<TableDashboard
				column_grids=vec![4, 4, 4]
				headings=vec![
					"Name".into_view(),
					"Expiry".into_view(),
					"Created At".into_view(),
				]
				render_rows=view! {
					<For
						each=move || data.get()
						key=|state| state.name.clone()
						let:child
					>
						<ApiTokenCard
							token=child
						/>
					</For>
				}.into_view()
			/>
		</div>
	}
}

mod api_token_card;
mod edit_token;

pub use self::{api_token_card::*, edit_token::*};
