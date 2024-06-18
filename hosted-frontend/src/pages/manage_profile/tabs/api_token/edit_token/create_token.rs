use leptos_use::{use_cookie, utils::FromToStringCodec};

use crate::{
	pages::{PermissionCard, TokenModal},
	prelude::*,
};

#[component]
pub fn CreateApiToken() -> impl IntoView {
	let create_api_token_action = create_server_action::<CreateApiTokenFn>();
	// let response = create_api_token_action.value();

	let (access_token, _) = use_cookie::<String, FromToStringCodec>(constants::ACCESS_TOKEN);
	let access_token_signal = move || access_token.get();
	let workspace_list = create_resource(access_token_signal, move |value| async move {
		list_user_workspace(value).await
	});

	let response = create_api_token_action.value();

	view! {
		{
			move || match response.get() {
				Some(thing) => match thing {
					Ok(data) => {
						logging::log!("logging response get {:#?}", data);
						view! {
							<TokenModal is_regenerated={false} token={data.token}/>
						}.into_view()
					},
					Err(_) => view! {}.into_view()
				},
				None => view! {}.into_view()
			}
		}
		<ActionForm action={create_api_token_action}  class="full-width fit-wide-screen full-height txt-white fc-fs-fs px-md">
			<input type="hidden" name="access_token" prop:value={access_token}/>

			<div class="fr-fs-ct mb-md full-width">
				<p class="txt-md">
					<strong class="txt-md">"Create new API Token"</strong>
				</p>
			</div>

			<div class="flex mb-xs full-width mb-md">
				<div class="flex-col-2 fr-fs-fs pt-xs">
					<label html_for="name" class="txt-white txt-sm">
						"Token Name"
					</label>
				</div>

				<div class="flex-col-10 fc-fs-fs pl-xl">
					<Input
						r#type={InputType::Text}
						placeholder="Enter Token Name"
						class="full-width"
						name="token_name"
						id="token_name"
					/>
				</div>
			</div>

			<div class="flex mb-xs full-width mb-md">
				<div class="flex-col-2 fc-fs-fs pt-xs">
					<label html_for="allowedIps" class="txt-white txt-sm">
						"Allowed IP(s)"
					</label>
					<small class="txt-xxs txt-grey">
						"By default, all IP addresses will be allowed."
					</small>
				</div>
				<div class="flex-col-10 fc-fs-fs pl-xl">
					<Input
						r#type={InputType::Text}
						placeholder="Enter Allowed IP addresses"
						class="full-width"
					/>
				</div>
			</div>

			<div class="flex mb-xs full-width mb-md">
				<div class="flex-col-2 fc-fs-fs pt-xs">
					<label html_for="tokenNbf" class="txt-white txt-sm">
						"Token Validity"
					</label>
					<small class="txt-xxs txt-grey">
						"By default, the token will be valid forever from the date created."
					</small>
				</div>
				<div class="flex-col-10 fr-fs-ct pl-xl">
					<div class="flex-col-1 fr-ct-ct txt-sm">"Valid from"</div>
					<div class="flex-col-5 fr-fs-fs pl-md">
						<Input
							r#type={InputType::Date}
							placeholder="Valid From"
							class="full-width cursor-text"
							name="token_nbf"
							id="token_nbf"
						/>
					</div>
					<div class="flex-col-1 fr-ct-ct txt-sm">"to"</div>
					<div class="flex-col-5 fr-fs-fs pl-md">
						<Input
							r#type={InputType::Date}
							placeholder="Valid Till"
							class="full-width cursor-text"
							name="token_exp"
							id="token_exp"
						/>
					</div>
				</div>
			</div>

			<div class="fc-fs-fs mb-xs full-width my-md gap-sm">
				<label class="txt-white txt-sm">"Choose Permissions"</label>
				<div class="full-width fc-fs-fs gap-xl">
					<Transition>
						{
							move || match workspace_list.get() {
								Some(workspace_list) => {
									match workspace_list {
										Ok(data)  => {
											data.workspaces.into_iter()
												.map(|workspace| view! {
													<PermissionCard workspace={workspace} />
												})
												.collect_view()
										},
										Err(_) => view! {}.into_view()
									}
								},
								None => view! {}.into_view()
							}
						}
					</Transition>
				</div>
			</div>

			<div class="full-width fr-fe-ct py-md mt-auto">
				<Link r#type={Variant::Link} to="/user/api-tokens" class="txt-sm txt-medium mr-sm">"BACK"</Link>
				<Link should_submit={true} r#type={Variant::Button} style_variant={LinkStyleVariant::Contained} class="txt-sm txt-medium mr-sm">
					"Create"
				</Link>
			</div>
		</ActionForm>
	}
}
