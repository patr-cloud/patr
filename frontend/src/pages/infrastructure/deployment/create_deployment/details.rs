use super::{DeploymentInfo, DetailsPageError};
use crate::{prelude::*, queries::list_runners_query};

#[component]
fn RunnerDropdown() -> impl IntoView {
	let deployment_info = expect_context::<RwSignal<DeploymentInfo>>();
	let runners_list = list_runners_query();

	view! {
		<InputDropdown
			placeholder="Choose A Runner"
			class="w-full"
			value={deployment_info
				.with(|info| info.runner_id.clone().map(|id| id.to_string()).unwrap_or_default())}
			on_select={move |id: String| {
				if let Ok(runner_id) = Uuid::parse_str(id.as_str()) {
					deployment_info.update(|info| info.runner_id = Some(runner_id))
				}
			}}
			options={Signal::derive(move || match runners_list.get() {
				Some(Ok(data)) => {
					data.runners
						.iter()
						.map(|x| InputDropdownOption {
							id: x.id.to_string(),
							disabled: false,
							label: x.name.clone(),
						})
						.collect::<Vec<_>>()
				}
				_ => vec![],
			})}
		/>
	}
}

/// The Deploy Details Page, has stuff like name, runner, registry, etc.
#[component]
pub fn DeploymentDetails(
	/// The Errors For This Page
	#[prop(into)]
	errors: MaybeSignal<DetailsPageError>,
) -> impl IntoView {
	let app_type = expect_context::<AppType>();
	let deployment_info = expect_context::<RwSignal<DeploymentInfo>>();

	let store_errors = store_value(errors);

	view! {
		<form id="details-form"></form>
		<div class="flex flex-col items-start justify-start w-full fit-wide-screen px-xl mt-xl">
			<h4 class="text-white text-lg pb-md">"Deployment Details"</h4>

			<div class="flex flex-col items-start justify-start w-full h-full text-white">
				<div class="flex mb-xs w-full">
					<div class="flex-2 flex items-center justify-start">
						<label
							html_for="name"
							class="text-white text-sm flex items-center justify-start"
						>
							"Name"
						</label>
					</div>

					<div class="flex-10 flex flex-col items-start justify-start">
						<Input
							placeholder="Deployment Name"
							r#type={InputType::Text}
							class="w-full"
							name="name"
							id="name"
							form={Some(String::from("details-form"))}
							value={Signal::derive(move || {
								deployment_info.get().name.unwrap_or_default()
							})}
							on_input={Box::new(move |ev| {
								ev.prevent_default();
								deployment_info
									.update(|info| info.name = Some(event_target_value(&ev)))
							})}
						/>

						<Show when={move || {
							store_errors.with_value(|errors| !errors.get().name.clone().is_empty())
						}}>
							<Alert r#type={AlertType::Error} class="mt-xs">
								{move || {
									store_errors.with_value(|errors| errors.get().name.clone())
								}}
							</Alert>
						</Show>
					</div>
				</div>

				<div class="flex my-xs w-full mb-md">
					<div class="flex-2 flex justify-start items-center">
						<label class="text-white text-sm flex justify-start items-center">
							"Registry"
						</label>
					</div>

					<div class="flex-10 flex flex-col items-start justify-start">
						<InputDropdown
							placeholder="Registry Name"
							value={deployment_info
								.with(|info| info.registry_name.clone().unwrap_or_default())}
							class="w-full"
							on_select={move |id: String| {
								deployment_info.update(|info| info.registry_name = Some(id))
							}}
							options={vec![
								InputDropdownOption {
									id: "docker.io".to_string(),
									label: "Docker Hub (docker.io)".to_string(),
									disabled: false,
								},
								InputDropdownOption {
									id: "registry.patr.cloud".to_string(),
									label: "Container Registry (registry.patr.cloud)".to_string(),
									disabled: false,
								},
							]}
						/>
					</div>
				</div>

				<div class="flex my-xs w-full">
					<div class="flex-2 flex justify-start items-center">
						<label class="text-white text-sm flex justify-start items-center">
							"Image Details"
						</label>
					</div>

					<div class="flex-6 flex flex-col items-start justify-start">
						<Input
							placeholder="Enter Repository Image Name"
							r#type={InputType::Text}
							name="repository_name"
							class="w-full"
							id="repository_name"
							form={Some(String::from("details-form"))}
							value={Signal::derive(move || {
								deployment_info.get().image_name.unwrap_or_default()
							})}
							on_input={Box::new(move |ev| {
								ev.prevent_default();
								deployment_info
									.update(|info| info.image_name = Some(event_target_value(&ev)))
							})}
						/>

						<Show when={move || {
							store_errors
								.with_value(|errors| !errors.get().image_name.clone().is_empty())
						}}>
							<Alert r#type={AlertType::Error} class="mt-xs">
								{move || {
									store_errors
										.with_value(|errors| errors.get().image_name.clone())
								}}
							</Alert>
						</Show>
					</div>

					<div class="flex-4 pl-md flex flex-col items-start justify-start">
						<Input
							r#type={InputType::Text}
							placeholder="Choose Image Tag"
							class="w-full"
							form={Some(String::from("details-form"))}
							name="image_tag"
							id="image_tag"
							value={Signal::derive(move || {
								deployment_info.get().image_tag.unwrap_or_default()
							})}
							on_input={Box::new(move |ev| {
								ev.prevent_default();
								deployment_info
									.update(|info| info.image_tag = Some(event_target_value(&ev)))
							})}
						/>

						<Show when={move || {
							store_errors
								.with_value(|errors| !errors.get().image_tag.clone().is_empty())
						}}>
							<Alert r#type={AlertType::Error} class="mt-xs">
								{move || {
									store_errors.with_value(|errors| errors.get().image_tag.clone())
								}}
							</Alert>
						</Show>
					</div>
				</div>
				{app_type
					.is_managed()
					.then(|| {
						view! {
							<div class="flex my-xs w-full mb-md">
								<div class="flex-2 flex justify-start items-center">
									<label class="text-white text-sm flex justify-start items-center">
										"Choose Runner"
									</label>
								</div>

								<div class="flex-10 flex flex-col items-start justify-start">
									<Transition>
										{move || {
											view! {
												<RunnerDropdown />

												<Show when={move || {
													store_errors
														.with_value(|errors| {
															!errors.get().runner.clone().is_empty()
														})
												}}>
													<Alert r#type={AlertType::Error} class="mt-xs">
														{move || {
															store_errors
																.with_value(|errors| errors.get().runner.clone())
														}}
													</Alert>
												</Show>
											}
												.into_view()
										}}
									</Transition>
								</div>
							</div>
						}
							.into_view()
					})}
			</div>
		</div>
	}
}
