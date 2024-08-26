use std::collections::BTreeMap;

use codee::string::FromToStringCodec;
use ev::MouseEvent;
use leptos_use::use_cookie;
use models::{api::user::UserApiToken, rbac::WorkspacePermission};
use time::{
	error::{Parse, TryFromParsed},
	macros::format_description,
	Date,
	OffsetDateTime,
};

use crate::prelude::*;

mod choose_permission;
mod create_token;
mod permission_card;
mod permission_item;
mod revoke_regen;
mod token_info;
mod token_modal;

pub use self::{
	choose_permission::*,
	create_token::*,
	permission_card::*,
	permission_item::*,
	revoke_regen::*,
	token_info::*,
	token_modal::*,
};

#[derive(Params, PartialEq)]
pub struct TokenParams {
	token_id: Option<String>,
}

/// Convert OffsetDateTime to a string date
pub fn convert_offset_to_date(date_time: Option<OffsetDateTime>) -> String {
	if date_time.is_some() {
		date_time.unwrap().date().to_string()
	} else {
		"".to_string()
	}
}

/// Convert String to OffsetDateTime
pub fn convert_string_to_datetime(dt_str: String) -> Result<OffsetDateTime, Parse> {
	let format = format_description!("[year]-[month]-[day]");
	let date = Date::parse(dt_str.as_str(), format);
	let mut date_time = OffsetDateTime::UNIX_EPOCH;
	logging::log!("{} {:?}", dt_str, date);

	if let Ok(date) = date {
		date_time = date_time.replace_date(date);
	} else {
		logging::log!("cannot parse date convert_string_to_datetime");
		return Err(Parse::TryFromParsed(TryFromParsed::InsufficientInformation));
	}

	Ok(date_time)
}

#[component]
fn EditApiTokenPermission() -> impl IntoView {
	// let api_token = create_rw_signal(api_token.get_untracked());
	let (access_token, _) = use_cookie::<String, FromToStringCodec>(constants::ACCESS_TOKEN);
	let api_token = expect_context::<ApiTokenInfo>().0;

	let workspace_list = create_resource(
		move || access_token.get(),
		move |value| async move { list_user_workspace(value).await },
	);

	move || match api_token.get() {
		Some(api_token) => view! {
			<div class="flex flex-col items-start justify-start mb-xs w-full my-md gap-sm">
				<label class="text-white text-sm">"Choose Permissions"</label>
				<div class="w-full fc-fs-fs gap-xl">
					{
						let api_token = api_token.clone();
						move || {
							match workspace_list.get() {
								Some(Ok(data)) => {
									data.workspaces.into_iter()
										.map(|workspace| {
											view! {
												<PermissionCard
													workspace={workspace}
												/>
											}
										})
										.collect_view()
								},
								_ => view! {
									<div>"Cannot Load Resource"</div>
								}.into_view()
							}
						}
					}
				</div>
			</div>
		}
		.into_view(),
		None => view! {
			<p>"Couldn't Load Resource!"</p>
		}
		.into_view(),
	}
}

#[derive(Copy, Clone)]
pub struct ApiTokenInfo(RwSignal<Option<WithId<UserApiToken>>>);

/// The Edit API Token Page
#[component]
pub fn EditApiToken() -> impl IntoView {
	let (access_token, _) = use_cookie::<String, FromToStringCodec>(constants::ACCESS_TOKEN);
	let navigate = leptos_router::use_navigate();

	let params = use_params::<TokenParams>();
	let token_id = create_rw_signal(params.with(|params| {
		params
			.as_ref()
			.map(|param: &TokenParams| param.token_id.clone().unwrap_or_default())
			.unwrap_or_default()
	}));

	let token_info = create_resource(
		move || (access_token.get(), token_id.get()),
		move |(access_token, token_id)| async move { get_api_token(access_token, token_id).await },
	);

	let token_info_signal = create_rw_signal::<Option<WithId<UserApiToken>>>(None);
	provide_context(ApiTokenInfo(token_info_signal));

	let on_submit = move |_: MouseEvent| {
		let navigate = navigate.clone();
		spawn_local(async move {
			if let Some(token_info) = token_info_signal.get() {
				let x = update_api_token(
					access_token.get(),
					token_id.get(),
					Some(token_info.name.clone()),
					Some(convert_offset_to_date(token_info.token_exp)),
					Some(convert_offset_to_date(token_info.token_nbf)),
					Some(token_info.permissions.clone()),
				)
				.await;

				if x.is_ok() {
					navigate("/user/api-tokens", Default::default());
				}
			}
		});
	};

	let permissions = create_rw_signal(BTreeMap::<Uuid, WorkspacePermission>::new());

	view! {
		<div class="w-full fit-wide-screen h-full text-white flex flex-col items-start justify-start px-md">
			<input type="hidden" name="access_token" prop:value={access_token}/>
			<input type="hidden" name="token_id" prop:value={token_id}/>

			<div class="flex justify-between items-center mb-md w-full">
				<p class="text-md">
					<strong class="text-md">"Manage Token"</strong>
				</p>

				<div class="flex justify-start items-center gap-md">
					<RegenerateApiToken />
					<RevokeApiToken />
				</div>
			</div>

			<form class="w-full h-full">
				<Transition>
					{
						move || match token_info.get() {
							Some(token_info) => {
								match token_info {
									Ok(data) => {
										let token = data.token.clone();
										token_info_signal.set(Some(data.token.clone()));
										view! {
											<TokenInfo />
											<EditApiTokenPermission/>
										}.into_view()
									},
									Err(err) => view! {
										<div>"Cannot Load Resource"</div>
									}.into_view()
								}
							},
							None => view! {}.into_view()
						}
					}
				</Transition>

				<div class="w-full flex justify-end items-center py-md mt-auto">
					<Link class="text-sm text-medium mr-sm">"BACK"</Link>
					<button
						type="submit"
						class="text-sm text-medium mr-sm flex justify-center items-center btn btn-primary"
						on:click={on_submit}
					>
						"UPDATE"
					</button>
				</div>
			</form>
		</div>
	}
}
