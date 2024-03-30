use std::str::FromStr;

use leptos_use::{use_cookie, utils::FromToStringCodec};
use models::api::user::{
	ListApiTokensPath,
	ListApiTokensRequest,
	ListApiTokensRequestHeaders,
	ListApiTokensResponse,
};

use crate::prelude::*;

#[component(transparent)]
pub fn ProfileRoutes() -> impl IntoView {
	view! {
		<Route path=LoggedInRoute::Profile view=ManageProfile>
			<Route path=LoggedInRoute::ApiTokens view=ApiTokensTab />
			<Route path=AppRoutes::Empty view=ProfileSettings />
		</Route>
	}
}

#[server]
pub async fn load_api_tokens_list(
	access_token: Option<String>,
) -> Result<Result<ListApiTokensResponse, ErrorType>, ServerFnError> {
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
pub fn ManageProfile() -> impl IntoView {
	let (access_token, _) = use_cookie::<String, FromToStringCodec>("access_token");
	// let access_token_signal = move || access_token.get();
	// let user_data = create_resource(access_token_signal, move |value| async move
	// { 	let some = load_api_tokens_list(value).await;
	// 	logging::log!("{:#?}", some.unwrap().unwrap());
	// });

	view! {
		<ContainerMain class="full-width full-height mb-md">
			<ContainerHead>
				<PageTitleContainer>
					<PageTitle>"Manage Profile"</PageTitle>
				</PageTitleContainer>

				<Tabs
					tab_items=vec![
						TabItem {
							name: "Settings".to_owned(),
							path: "".to_owned()
						},
						TabItem {
							name: "API Tokens".to_owned(),
							path: "api-tokens".to_owned()
						},
					]
				/>
			</ContainerHead>

			<ContainerBody class="gap-md">
				<Outlet />
			</ContainerBody>
		</ContainerMain>
	}
}

mod tabs;

pub use self::tabs::*;
