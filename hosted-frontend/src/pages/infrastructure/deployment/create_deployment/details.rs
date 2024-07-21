use leptos_use::{use_cookie, utils::FromToStringCodec};

use super::{DeploymentInfo, DetailsPageError};
use crate::prelude::*;

/// The Deploy Details Page, has stuff like name, runner, registry, etc.
#[component]
pub fn DeploymentDetails(
	/// The Errors For This Page
	#[prop(into)]
	errors: MaybeSignal<DetailsPageError>,
) -> impl IntoView {
	let deployment_info = expect_context::<RwSignal<DeploymentInfo>>();

	let (access_token, _) = use_cookie::<String, FromToStringCodec>(constants::ACCESS_TOKEN);
	let (current_workspace_id, _) =
		use_cookie::<String, FromToStringCodec>(constants::LAST_USED_WORKSPACE_ID);

	let runner_list = create_resource(
		move || (access_token.get(), current_workspace_id.get()),
		move |(access_token, workspace_id)| async move {
			list_runners(workspace_id, access_token).await
		},
	);

	let store_errors = store_value(errors);

	view! {
		<div class="fc-fs-fs full-width fit-wide-screen px-xl mt-xl">
			<h4 class="txt-white txt-lg pb-md txt-white">"Deployment Details"</h4>

			<div class="fc-fs-fs full-width full-height txt-white">
				<div class="flex mb-xs full-width">
					<div class="flex-col-2 fr-fs-ct">
						<label html_for="name" class="txt-white txt-sm fr-fs-ct">"Name"</label>
					</div>

					<div class="flex-col-10 fc-fs-fs">
						<Input
							placeholder="Deployment Name"
							r#type=InputType::Text
							class="full-width"
							name="name"
							id="name"
							value={Signal::derive(move || deployment_info.get().name.unwrap_or_default())}
						/>
					</div>
				</div>

				<div class="flex my-xs full-width mb-md">
					<div class="flex-col-2 fr-fs-ct">
						<label class="txt-white txt-sm fr-fs-ct">"Registry"</label>
					</div>

					<ul class="flex-col-10 fr-fs-fs gap-sm">
						<li class="flex-col-6">
							<label html_for="registry_name" class="
								bg-secondary-light fr-fs-ct gap-md full-width txt-white full-width flex-col-4 br-sm py-sm px-xl
							">
								<input name="registry_name" value="docker" type="radio" />
								<p>"Docker Registry"</p>
							</label>
						</li>

						<li class="flex-col-6">
							<label html_for="registry_name" class="
								bg-secondary-light fr-fs-ct gap-md full-width txt-white full-width flex-col-4 br-sm py-sm px-xl
							">
								<input name="registry_name" value="patr" type="radio" />
								<p>"Patr Registry"</p>
							</label>
						</li>
					</ul>
				</div>

				<div class="flex my-xs full-width">
					<div class="flex-col-2 fr-fs-ct">
						<label class="txt-white txt-sm fr-fs-ct">"Image Details"</label>
					</div>

					<div class="flex-col-6 fc-fs-fs">
						<Input
							placeholder="Enter Repository Image Name"
							r#type={InputType::Text}
							name="image_name"
							class="full-width"
							id="repository_name"
						/>
					</div>

					<div class="flex-col-4 pl-md fc-fs-fs">
						<Input
							r#type=InputType::Text
							placeholder="Choose Image Tag"
							class="full-width"
							name="image_tag"
							id="image_tag"
						/>
					</div>
				</div>

				<div class="flex my-xs full-width mb-md">
					<div class="flex-col-2 fr-fs-ct">
						<label class="txt-white txt-sm fr-fs-ct">"Choose Runner"</label>
					</div>

					<div class="flex-col-10 fc-fs-fs">
						<Transition>
							<ul class="fr-fs-fs gap-sm full-width f-wrap">
								{
									move || match runner_list.get() {
										Some(Ok(runners)) => view! {
											<For
												each={move || runners.runners.clone()}
												key={|state| state.id.clone()}
												let:child
											>
												 <li class="flex-col-3">
													<label class="fr-fs-ct gap-md bg-secondary-light fr-fs-ct full-width txt-white br-sm py-sm px-xl">
														<input
															type="radio"
															value={child.id.to_string()}
															name="runner"
														/>
														<p>{child.name.clone()}</p>
													</label>
												</li>
											</For>
										}.into_view(),
										_ => view! {
											<li class="px-xl py-sm ul-light fr-fs-ct full-width br-bottom-sm txt-white">
												"Error Loading Runners"
											</li>
										}.into_view()
									}
								}
							</ul>
						</Transition>
					</div>
				</div>
			</div>
		</div>
	}
}
