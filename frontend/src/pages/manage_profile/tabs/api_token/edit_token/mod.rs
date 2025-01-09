use std::collections::BTreeMap;

use ev::MouseEvent;
use models::{
	api::user::{UpdateApiTokenRequest, UserApiToken},
	rbac::WorkspacePermission,
};
use time::{
	error::{Parse, TryFromParsed},
	macros::format_description,
	Date,
	OffsetDateTime,
};

use crate::{prelude::*, queries::get_api_token_query};

mod revoke_regen;
mod token_info;

use self::{revoke_regen::*, token_info::*};
use super::{
	components::PermissionCard,
	utils::{ApiTokenInfo, ApiTokenPermissions, CreateApiTokenInfo},
};

/// Path URL Params for the Edit API Token Page
#[derive(Params, PartialEq)]
pub struct TokenParams {
	/// The ID of the API Token
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
	let (access_token, _) = AuthState::load();
	let api_token = expect_context::<ApiTokenInfo>().0;

	let workspace_list = create_resource(
		move || access_token.get().get_access_token(),
		move |value| async move {
			if let Some(value) = value {
				list_user_workspace(value).await
			} else {
				Err(ServerFnError::WrappedServerError(ErrorType::Unauthorized))
			}
		},
	);

	move || match api_token.get() {
		Some(_) => view! {
			<div class="flex flex-col items-start justify-start mb-xs w-full my-md gap-sm">
				<label class="text-white text-sm">"Choose Permissions"</label>
				<div class="w-full fc-fs-fs gap-xl">
					<Transition>
						{move || {
							match workspace_list.get() {
								Some(Ok(data)) => {
									data.workspaces
										.into_iter()
										.map(|workspace| {
											view! { <PermissionCard workspace={workspace} /> }
										})
										.collect_view()
								}
								_ => view! { <div>"Cannot Load Resource"</div> }.into_view(),
							}
						}}
					</Transition>
				</div>
			</div>
		}
		.into_view(),
		None => view! { <p>"Loading..."</p> }.into_view(),
	}
}

/// The Edit API Token Page
#[component]
pub fn EditApiToken() -> impl IntoView {
	let (access_token, _) = AuthState::load();

	let params = use_params::<TokenParams>();
	let token_id = Signal::derive(move || {
		params.with(|params| {
			params
				.as_ref()
				.map(|param: &TokenParams| param.token_id.clone().unwrap_or_default())
				.unwrap_or_default()
				.parse::<Uuid>()
				.unwrap()
		})
	});

	let token_info = get_api_token_query(token_id);

	let token_info_signal = create_rw_signal::<Option<WithId<UserApiToken>>>(None);
	let api_token_changes = create_rw_signal(CreateApiTokenInfo::new());
	let token_permissions = create_rw_signal::<Option<BTreeMap<Uuid, WorkspacePermission>>>(None);

	provide_context(api_token_changes);
	provide_context(ApiTokenInfo(token_info_signal));
	provide_context(ApiTokenPermissions(token_permissions));

	create_effect(move |_| match token_info.get() {
		Some(Ok(data)) => {
			token_info_signal.set(Some(data.token.clone()));
			token_permissions.set(Some(data.token.permissions.clone()));
		}
		_ => {}
	});

	let on_submit = move |ev: MouseEvent| {
		ev.prevent_default();

		spawn_local(async move {
			if let Some(token_info) = token_info_signal.get() {
				logging::log!(
					"token_info_signal {:?} {:?}",
					token_info,
					token_permissions.get()
				);
				let res = update_api_token(
					access_token.get_untracked().get_access_token(),
					token_id.get_untracked(),
					UpdateApiTokenRequest {
						name: api_token_changes.get().name.clone(),
						token_nbf: api_token_changes.get().token_nbf,
						token_exp: api_token_changes.get().token_exp,
						permissions: token_permissions.get(),
						allowed_ips: None,
					},
				)
				.await;
				logging::log!("x {:?} {:?}", res, token_info.permissions.clone());
			}
		});
	};

	view! {
		<div class="w-full fit-wide-screen h-full text-white flex flex-col items-start justify-start px-md">
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
				<TokenInfo />
				<EditApiTokenPermission />

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
