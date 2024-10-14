use std::rc::Rc;

use ev::MouseEvent;
use models::api::workspace::deployment::*;

use super::DeploymentInfoContext;
use crate::{
	pages::ShowWorkspaceInfoPropsBuilder_Error_Missing_required_field_workspace,
	prelude::*,
	queries::{delete_deployment_query, start_deployment_query, stop_deployment_query},
};

/// The component that contains the delete dialog for a deployment.
#[component]
pub fn DeleteDialog(
	/// The Deployment Name
	#[prop(into)]
	deployment_name: MaybeSignal<String>,
	/// The Deployment Id
	#[prop(into)]
	deployment_id: MaybeSignal<Uuid>,
	/// The Modal Control Signal
	#[prop(into)]
	show_delete_dialog: RwSignal<bool>,
) -> impl IntoView {
	let delete_deployment_action = delete_deployment_query();

	let input_value = create_rw_signal("".to_string());
	let deployment_name = Signal::derive(move || deployment_name.get());

	let is_name_matching = Signal::derive(move || input_value.get() == deployment_name.get());

	let on_click_delete = move |ev: &MouseEvent| {
		ev.prevent_default();
		if is_name_matching.get() {
			delete_deployment_action.dispatch(deployment_id.get());
		}
	};

	view! {
		<Modal color_variant={SecondaryColorVariant::Light}>
			<div
				style="border-radius:1rem"
				class="p-xl bg-secondary-light text-white flex flex-col items-center justify-start h-[35vh] gap-lg w-2/5"
			>
				<div class="flex justify-between items-center w-full">
					<h1 class="text-md text-primary">"Delete "{deployment_name}</h1>
					<Link
						on_click={Rc::new(move |_| {
							show_delete_dialog.set(false);
						})}
					>
						<Icon size={Size::ExtraSmall} icon={IconType::X} />
					</Link>
				</div>
				<p class="font-bold text-md">"Unexpected things will happen if you don't read this."</p>
				<p>"This will Permanently delete the deployment and all of its data, history, logs and configuration."</p>

				<label>
					<p>{move || format!("To Confirm, type \"{}\" in the box below", deployment_name.get())}</p>
					<Input
						variant={SecondaryColorVariant::Medium}
						required={true}
						value={input_value}
						on_input={Box::new(move |ev| {
							input_value.set(event_target_value(&ev));
						})}
					/>
				</label>
				<Link
					on_click={Rc::new(on_click_delete)}
					r#type={Variant::Button}
					disabled={Signal::derive(move || !is_name_matching.get())}
					style_variant={LinkStyleVariant::Contained}
					color={Color::Error}
				>
					"DELETE THIS DEPLOYMENT"
				</Link>
			</div>
		</Modal>
	}
}

/// The component that contains the start/stop and delete buttons for a
/// deployment.
#[component]
pub fn StartStopButton() -> impl IntoView {
	let deployment_info = expect_context::<DeploymentInfoContext>().0;

	let show_delete_dialog = create_rw_signal(false);

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
		show_delete_dialog.set(true);
	};

	move || match deployment_info.get() {
		Some(deployment_info) => view! {
			<Link
				clone:deployment_info
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
						deployment_info.clone().deployment.clone().status.clone(),
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
						deployment_info.deployment.clone().status.clone(),
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
			<Show when={move || show_delete_dialog.get()}>
				<DeleteDialog
					deployment_name={deployment_info.clone().deployment.clone().name.clone()}
					deployment_id={deployment_info.deployment.clone().id.clone()}
					show_delete_dialog={show_delete_dialog}
				/>
			</Show>
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
