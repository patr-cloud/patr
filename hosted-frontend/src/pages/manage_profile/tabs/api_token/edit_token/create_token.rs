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
				Some(data) => match data {
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
		<ActionForm action={create_api_token_action}
			class="w-full fit-wide-screen h-full px-md \
				text-white flex flex-col items-start justify-start"
		>
			<input type="hidden" name="access_token" prop:value={access_token}/>

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
					/>
				</div>
			</div>

			<div class="flex w-full mb-md">
				<div class="flex-2 flex flex-col items-start justify-start pt-xs">
					<label html_for="allowedIps" class="text-white text-sm">
						"Allowed IP(s)"
					</label>
					<small class="text-xxs text-grey">
						"By default, all IP addresses will be allowed."
					</small>
				</div>
				<div class="flex-10 flex flex-col items-start justify-start pl-xl">
					<Input
						r#type={InputType::Text}
						placeholder="Enter Allowed IP addresses"
						class="w-full"
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
						/>
					</div>
				</div>
			</div>

			<div class="flex flex-col items-start justify-start mb-xs w-full my-md gap-sm">
				<label class="text-white text-sm">"Choose Permissions"</label>
				<div class="w-full flex flex-col items-start justify-start gap-xl">
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

			<div class="w-full flex items-center justify-end py-md mt-auto">
				<Link r#type={Variant::Link} to="/user/api-tokens" class="text-sm text-medium mr-sm">"BACK"</Link>
				<Link should_submit={true} r#type={Variant::Button} style_variant={LinkStyleVariant::Contained} class="txt-sm txt-medium mr-sm">
					"Create"
				</Link>
			</div>
		</ActionForm>
	}
}
