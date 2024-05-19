use models::api::auth::*;

use crate::{pages::LoginPage, prelude::*};

#[allow(async_fn_in_trait)] // WIP
pub trait AppAPIs {
	async fn login(
		request: ApiRequest<LoginRequest>,
	) -> Result<AppResponse<LoginRequest>, ServerFnError<ErrorType>>;
}

#[component]
pub fn App() -> impl IntoView {
	view! { <LoginPage/> }
}
