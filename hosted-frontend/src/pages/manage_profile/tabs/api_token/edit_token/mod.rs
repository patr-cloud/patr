use leptos_use::{use_cookie, utils::FromToStringCodec};
use models::api::user::{ListUserWorkspacesResponse, UserApiToken};
use time::OffsetDateTime;

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

#[component]
fn EditApiTokenPermission(
	/// Workspace List
	workspace_list: Resource<
		Option<String>,
		Result<ListUserWorkspacesResponse, ServerFnError<ErrorType>>,
	>,
	/// The Api Token
	#[prop(into)]
	api_token: MaybeSignal<WithId<UserApiToken>>,
) -> impl IntoView {
	view! {
		<div class="fc-fs-fs mb-xs full-width my-md gap-sm">
			<label class="txt-white txt-sm">"Choose Permissions"</label>
			<div class="full-width fc-fs-fs gap-xl">
					{
						let api_token = api_token.clone();
						move || {
							match workspace_list.get() {
								Some(workspace_list) => {
									match workspace_list {
										Ok(data)  => {
											data.workspaces.into_iter()
												.map(|workspace| {
													let permissions = api_token.get().permissions.get(&workspace.id).map(|id| id.clone());
													// logging::log!("{:#?}", api_token.get().permissions.get(&workspace.id));
													view! {
														<PermissionCard
															permissions={permissions}
															workspace={workspace}
														/>
													}
												})
												.collect_view()
										},
										Err(err) => view! {
											<div>"Cannot Load Resource"</div>
										}.into_view()
									}
								},
								None => view! {
									<div>"Cannot Load Resource"</div>
								}.into_view()
							}
						}
					}
			</div>
		</div>
	}
}

/// The Edit API Token Page
#[component]
pub fn EditApiToken() -> impl IntoView {
	let (access_token, _) = use_cookie::<String, FromToStringCodec>(constants::ACCESS_TOKEN);

	let update_api_token_action = create_server_action::<UpdateApiTokenFn>();

	let params = use_params::<TokenParams>();
	let token_id = create_rw_signal(params.with(|params| {
		params
			.as_ref()
			.map(|param| param.token_id.clone().unwrap_or_default())
			.unwrap_or_default()
	}));

	let workspace_list = create_resource(
		move || access_token.get(),
		move |value| async move { list_user_workspace(value).await },
	);

	let token_info = create_resource(
		move || (access_token.get(), token_id.get()),
		move |(access_token, token_id)| async move { get_api_token(access_token, token_id).await },
	);

	create_effect(move |_| {
		logging::log!("{:#?}", token_info.get());
	});

	view! {
		<div class="full-width fit-wide-screen full-height txt-white fc-fs-fs px-md">
			<input type="hidden" name="access_token" prop:value={access_token}/>
			<input type="hidden" name="token_id" prop:value={token_id}/>

			<div class="fr-sb-ct mb-md full-width">
				<p class="txt-md">
					<strong class="txt-md">"Manage Token"</strong>
				</p>

				<div class="fr-fs-ct gap-md">
					<RegenerateApiToken />
					<RevokeApiToken />
				</div>
			</div>

			<ActionForm class="full-width full-height" action={update_api_token_action}>
				<Transition>
					{
						move || match token_info.get() {
							Some(token_info) => {
								match token_info {
									Ok(data) => {
										let token = data.token.clone();
										view! {
											<TokenInfo token_info={data.token} />
											<EditApiTokenPermission
												workspace_list={workspace_list}
												api_token={token}
											/>
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


				<div class="full-width fr-fe-ct py-md mt-auto">
					<Link class="txt-sm txt-medium mr-sm">"BACK"</Link>
					<Link
						r#type={Variant::Button}
						should_submit={true}
						style_variant={LinkStyleVariant::Contained}
						class="txt-sm txt-medium mr-sm"
					>
						"UPDATE"
					</Link>
				</div>
			</ActionForm>
		</div>
	}
}
