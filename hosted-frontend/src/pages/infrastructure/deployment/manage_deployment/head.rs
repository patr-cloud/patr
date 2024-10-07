use std::rc::Rc;

use ev::MouseEvent;
use models::api::workspace::deployment::*;

use super::DeploymentInfoContext;
use crate::{
	prelude::*,
	queries::{delete_deployment_query, start_deployment_query, stop_deployment_query},
};

/// The component that contains the start/stop and delete buttons for a
/// deployment.
#[component]
pub fn StartStopButton() -> impl IntoView {
	let deployment_info = expect_context::<DeploymentInfoContext>().0;

	let start_deployment_action = start_deployment_query();
	let stop_deployment_action = stop_deployment_query();
	let delete_deployment_action = delete_deployment_query();

	let on_click_start_stop = move |ev: &MouseEvent| {
		ev.prevent_default();
		if let Some(deployment_info) = deployment_info.get() {
			let status = deployment_info.deployment.status.clone();
			match status {
				DeploymentStatus::Running => {
					stop_deployment_action.dispatch(deployment_info.deployment.id.clone());
				}
				DeploymentStatus::Created | DeploymentStatus::Stopped => {
					start_deployment_action.dispatch(deployment_info.deployment.id.clone());
				}
				_ => {}
			}
		}
	};

	let on_click_delete = move |ev: MouseEvent| {
		ev.prevent_default();
		if let Some(deployment_info) = deployment_info.get() {
			delete_deployment_action.dispatch(deployment_info.deployment.id.clone());
		}
	};

	move || match deployment_info.get() {
		Some(deployment_info) => view! {
			<Link
				r#type={Variant::Button}
				on_click={Rc::new(move |ev: &MouseEvent| {
					on_click_start_stop(ev);
				})}
				style_variant={LinkStyleVariant::Contained}
				disabled={match deployment_info.deployment.status {
					DeploymentStatus::Running
					| DeploymentStatus::Created
					| DeploymentStatus::Stopped => false,
					_ => true,
				}}
			>
				<Icon
					icon={match Status::from_deployment_status(
						deployment_info.deployment.status.clone(),
					) {
						Status::Running => IconType::PauseCircle,
						_ => IconType::PlayCircle,
					}}
					size={Size::ExtraSmall}
					class="mr-xs"
					color={Color::Secondary}
				/>
				{
					let status = Status::from_deployment_status(
						deployment_info.deployment.status.clone(),
					);
					match status {
						Status::Running => "STOP",
						Status::Created | Status::Stopped => "START",
						_ => status.get_status_text(),
					}
				}
			</Link>

			<button
				class="flex items-center justify-start btn btn-error ml-md"
				on:click={on_click_delete}
			>
				<Icon icon={IconType::Trash2} size={Size::ExtraSmall} class="mr-xs" />
				"DELETE"
			</button>
		}
		.into_view(),
		None => ().into_view(),
	}
}

/// The Header Component for the deployment management page.
/// Contains the deployment name, the start/stop and delete buttons.
#[component]
pub fn ManageDeploymentHeader() -> impl IntoView {
	let deployment_info = expect_context::<DeploymentInfoContext>().0;

	view! {
		<ContainerHead>
			<PageTitleContainer
				page_title_items={Signal::derive(move || vec![
					PageTitleItem {
						title: "Infrastructure".to_owned(),
						link: None,
						icon_position: PageTitleIconPosition::End,
						variant: PageTitleVariant::Heading,
					},
					PageTitleItem {
						title: "Deployment".to_owned(),
						link: Some("/deployment".to_owned()),
						icon_position: PageTitleIconPosition::End,
						variant: PageTitleVariant::SubHeading,
					},
					PageTitleItem {
						title: match deployment_info.get() {
							Some(info) => info.deployment.name.clone(),
							None => "Loading...".to_string(),
						},
						link: None,
						icon_position: PageTitleIconPosition::None,
						variant: PageTitleVariant::Text,
					}
				])}
				action_buttons={
					Some(view! { <StartStopButton /> }.into_view())
				}
			>
			</PageTitleContainer>

			<Tabs tab_items={vec![
				TabItem {
					name: "Details".to_owned(),
					path: "".to_owned(),
				},
				TabItem {
					name: "Monitoring".to_owned(),
					path: "monitor".to_owned(),
				},
				TabItem {
					name: "Scaling".to_owned(),
					path: "scaling".to_owned(),
				},
				TabItem {
					name: "URLs".to_owned(),
					path: "urls".to_owned(),
				},
				TabItem {
					name: "Image History".to_owned(),
					path: "history".to_owned(),
				},
				TabItem {
					name: "Logs".to_owned(),
					path: "logs".to_owned(),
				},
			]} />
		</ContainerHead>
	}
}
