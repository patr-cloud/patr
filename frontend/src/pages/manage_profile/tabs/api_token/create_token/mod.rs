use std::collections::BTreeMap;

use ev::SubmitEvent;
use models::{
	api::user::{CreateApiTokenRequest, UserApiToken},
	rbac::WorkspacePermission,
};
use time::OffsetDateTime;

use super::{
	components::{PermissionCard, TokenModal},
	utils::{ApiTokenPermissions, CreateApiTokenInfo},
};
use crate::{
	pages::{convert_offset_to_date, convert_string_to_datetime},
	prelude::*,
	queries::create_api_token_query,
};

/// The Create API Token Page
#[component]
pub fn CreateApiToken() -> impl IntoView {
	let (state, _) = AuthState::load();
	let workspace_list = create_resource(
		move || state.get().get_access_token(),
		move |value| async move { list_user_workspace(value).await },
	);

	let create_api_token_action = create_api_token_query();
	let response = create_api_token_action.value();

	let api_token_info = create_rw_signal(CreateApiTokenInfo::new());
	let api_token_permissions =
		create_rw_signal::<Option<BTreeMap<Uuid, WorkspacePermission>>>(Some(BTreeMap::new()));

	provide_context(api_token_info);
	provide_context(ApiTokenPermissions(api_token_permissions));

	let on_submit_create = move |ev: SubmitEvent| {
		ev.prevent_default();
		logging::log!(
			"{:?}\n{:?}",
			api_token_info.get(),
			api_token_permissions.get()
		);

		if let Some(name) = api_token_info.get().name {
			let permissions = api_token_permissions
				.get()
				.expect("the permissions context to be Some");
			let request = CreateApiTokenRequest {
				token: UserApiToken {
					name,
					token_exp: api_token_info.get().token_exp,
					token_nbf: api_token_info.get().token_nbf,
					created: OffsetDateTime::now_utc(),
					allowed_ips: None,
					permissions,
				},
			};

			create_api_token_action.dispatch(request);
		} else {
			logging::error!("Invalid Api Token Info");
		}
	};

	view! {
		{move || match response.get() {
			Some(data) => {
				match data {
					Ok(data) => {
						logging::log!("logging response get {:#?}", data);
						view! { <TokenModal is_regenerated=false token={data.token} /> }.into_view()
					}
					Err(_) => view! {}.into_view(),
				}
			}
			None => view! {}.into_view(),
		}}
		<form
			on:submit={on_submit_create}
			class="w-full fit-wide-screen h-full px-md \
			text-white flex flex-col items-start justify-start"
		>
			<div class="flex justify-start items-center mb-md w-full">
				<p class="text-md">
					<strong class="text-md">"Create new API Token"</strong>
				</p>
			</div>

			<div class="flex w-full mb-md">
				<div class="flex-2 flex items-start justify-start pt-xs">
					<label html_for="name" class="text-white text-sm">
						"Token Name"
					</label>
				</div>

				<div class="flex-10 flex flex-col items-start justify-start pl-xl">
					<Input
						r#type={InputType::Text}
						placeholder="Enter Token Name"
						class="w-full"
						name="token_name"
						id="token_name"
						required={true}
						value={Signal::derive(move || api_token_info.get().name.clone().unwrap_or_default())}
						on_input={Box::new(move |ev| {
							ev.prevent_default();
							api_token_info.update(|token| {
								token.name = Some(event_target_value(&ev));
							});
						})}
					/>
				</div>
			</div>

			<div class="flex w-full mb-md">
				<div class="flex-2 flex flex-col items-start justify-start pt-xs">
					<label html_for="tokenNbf" class="text-white text-sm">
						"Token Validity"
					</label>
					<small class="text-xxs text-grey">
						"By default, the token will be valid forever from the date created."
					</small>
				</div>
				<div class="flex-10 flex justify-start items-center pl-xl">
					<div class="flex-1 flex items-center justify-center text-sm">"Valid from"</div>
					<div class="flex-5 flex items-start justify-start pl-md">
						<Input
							r#type={InputType::Date}
							placeholder="Valid From"
							class="w-full cursor-text"
							name="token_nbf"
							id="token_nbf"
							value={Signal::derive(move || convert_offset_to_date(api_token_info.get().token_nbf))}
							on_input={Box::new(move |ev| {
								ev.prevent_default();
								api_token_info.update(|token| {
									token.token_nbf = convert_string_to_datetime(event_target_value(&ev)).ok();
								})
							})}
						/>
					</div>
					<div class="flex-1 flex items-center justify-center text-sm">"to"</div>
					<div class="flex-5 flex items-start justify-start pl-md">
						<Input
							r#type={InputType::Date}
							placeholder="Valid Till"
							class="w-full cursor-text"
							name="token_exp"
							id="token_exp"
							value={Signal::derive(move || convert_offset_to_date(api_token_info.get().token_exp))}
							on_input={Box::new(move |ev| {
								ev.prevent_default();
								api_token_info.update(|token| {
									token.token_exp = convert_string_to_datetime(event_target_value(&ev)).ok()
								});
							})}
						/>
					</div>
				</div>
			</div>

			<div class="flex flex-col items-start justify-start mb-xs w-full my-md gap-sm">
				<label class="text-white text-sm">"Choose Permissions"</label>
				<div class="w-full flex flex-col items-start justify-start gap-xl">
					<Transition>
						{move || match workspace_list.get() {
							Some(Ok(workspace_list)) => {
								workspace_list
									.workspaces
									.into_iter()
									.map(|workspace| {
										view! {
											<PermissionCard workspace={workspace} />
										}.into_view()
									})
									.collect_view()
							}
							Some(Err(_)) => {
								view! { <div>"Error loading workspaces"</div> }.into_view()
							}
							None => view! { <div>"Loading workspaces..."</div> }.into_view(),
						}}
					</Transition>
				</div>
			</div>

			<div class="w-full flex items-center justify-end py-md mt-auto">
				<Link
					r#type={Variant::Link}
					to="/user/api-tokens"
					class="text-sm text-medium mr-sm"
				>
					"BACK"
				</Link>
				<Link
					should_submit={true}
					r#type={Variant::Button}
					style_variant={LinkStyleVariant::Contained}
					class="txt-sm txt-medium mr-sm"
				>
					"Create"
				</Link>
			</div>
		</form>
	}
}
