use std::{collections::BTreeMap, rc::Rc};

use ev::MouseEvent;

use crate::{prelude::*, queries::create_deployment_query};

mod details;
mod head;
mod running;
mod scale;

pub use self::{details::*, head::*, running::*, scale::*};
pub use super::utils::{DeploymentInfo, DetailsPageError, Page, RunnerPageError, ScalePageError};

/// The Create Deployment Page
#[component]
pub fn CreateDeployment() -> impl IntoView {
	let app_type = expect_context::<AppType>();
	let deployment_info = create_rw_signal(DeploymentInfo {
		name: None,
		registry_name: Some("docker.io".to_string()),
		image_tag: None,
		image_name: None,
		runner_id: match app_type {
			AppType::Managed => None,
			AppType::SelfHosted => Some(Uuid::nil()),
		},
		machine_type: None,
		deploy_on_create: false,
		deploy_on_push: false,
		min_horizontal_scale: None,
		max_horizontal_scale: None,
		startup_probe: None,
		liveness_probe: None,
		environment_variables: BTreeMap::new(),
		ports: BTreeMap::new(),
		volumes: BTreeMap::new(),
	});

	provide_context(deployment_info);

	let page = create_rw_signal(Page::Details);

	let details_error = create_rw_signal(DetailsPageError::new());
	let runner_error = create_rw_signal(RunnerPageError::new());
	let scale_page_error = create_rw_signal(ScalePageError::new());

	let create_deployment_action = create_deployment_query();

	let on_submit = move |ev: MouseEvent| {
		ev.prevent_default();
		if let Some(deployment_info) = deployment_info.get().convert_to_deployment_req() {
			create_deployment_action.dispatch(deployment_info);
		} else {
			logging::error!("Invalid deployment info");
		}
	};

	let on_click_next = move |ev: MouseEvent| {
		ev.prevent_default();
		let deployment_info = deployment_info.get();

		match page.get() {
			Page::Details => {
				details_error.set(DetailsPageError::new());
				if deployment_info.name.is_none() {
					details_error.update(|errors| errors.name = "Name is Required!".to_string());
					return;
				}
				if deployment_info.registry_name.is_none() {
					details_error.update(|errors| errors.registry = "".to_string());
					return;
				}
				if deployment_info.image_name.is_none() {
					details_error
						.update(|errors| errors.image_name = "Image Name is Required".to_string());
					return;
				}
				if deployment_info.image_tag.is_none() {
					details_error.update(|errors| {
						errors.image_tag = "Image Tag is also Required!".to_string()
					});
					return;
				}
				if !deployment_info
					.runner_id
					.is_some_and(|x| !x.to_string().is_empty())
				{
					details_error
						.update(|errors| errors.runner = "Please Select a Runner".to_string());
					return;
				}
			}
			Page::Running => {
				if deployment_info.ports.is_empty() {
					runner_error.update(|errors| {
						errors.ports = "Please Select at least one Port".to_string()
					});
					return;
				}
			}
			Page::Scaling => {
				if deployment_info.machine_type.is_none() {
					scale_page_error.update(|errors| {
						errors.machine_type = "Please select a Machine Type".to_string()
					});
					return;
				}
			}
		};
		page.update(|x| *x = x.next());
	};

	view! {
		<CreateDeploymentHead />
		<ContainerBody class="gap-md overflow-y-auto px-md">
			{move || match page.get() {
				Page::Details => view! { <DeploymentDetails errors={details_error} /> }.into_view(),
				Page::Running => view! { <RunningDetails errors={runner_error} /> }.into_view(),
				Page::Scaling => view! { <ScaleDeployment /> }.into_view(),
			}}
			<div class="flex justify-end items-center gap-md w-full fit-wide-screen mx-auto mt-auto pt-md pb-xl px-md">
				<Show when={move || page.get() != Page::Details}>
					<Link
						on_click={Rc::new(move |_| {
							page.update(|x| *x = x.back());
						})}
						r#type={Variant::Button}
						style_variant={LinkStyleVariant::Plain}
					>
						"BACK"
					</Link>
				</Show>

				<Show
					when={move || page.get() == Page::Scaling}
					fallback={move || {
						view! {
							<button
								form={move || match page.get() {
									Page::Details => "details-form",
									Page::Running => "running-form",
									Page::Scaling => "scaling-form",
								}}
								type="submit"
								class="flex items-center justify-center btn btn-primary"
								on:click={on_click_next}
							>
								"NEXT"
								<Icon
									icon={IconType::ChevronRight}
									size={Size::ExtraSmall}
									color={Color::Black}
									class="ml-xxs"
								/>
							</button>
						}
					}}
				>
					<button
						type="submit"
						class="flex items-center justify-center btn btn-primary"
						on:click={on_submit}
					>
						"CREATE"
					</button>
				</Show>
			</div>
		</ContainerBody>
	}
}
