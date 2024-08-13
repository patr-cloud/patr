use std::rc::Rc;

use ev::MouseEvent;
use models::api::workspace::deployment::*;
use codee::string::FromToStringCodec;

use crate::{pages::DeploymentInfoContext, prelude::*};

#[component]
pub fn StartStopButton() -> impl IntoView {
	let deployment_info = expect_context::<DeploymentInfoContext>().0;
	let (access_token, _) = use_cookie::<String, FromToStringCodec>(constants::ACCESS_TOKEN);
	let (current_workspace_id, _) =
		use_cookie::<String, FromToStringCodec>(constants::LAST_USED_WORKSPACE_ID);

	let on_click_start_stop = move |_: &MouseEvent| {
		spawn_local(async move {
			if let Some(deployment_info) = deployment_info.get() {
				let status = deployment_info.deployment.status.clone();
				match status {
					DeploymentStatus::Running => {
						stop_deployment(
							access_token.get(),
							Some(deployment_info.deployment.id.to_string()),
							current_workspace_id.get(),
						)
						.await;
					}
					DeploymentStatus::Created | DeploymentStatus::Stopped => {
						start_deployment(
							access_token.get(),
							Some(deployment_info.deployment.id.to_string()),
							current_workspace_id.get(),
						)
						.await;
					}
					_ => {}
				}
			}
		})
	};

	let on_click_delete = move |_: MouseEvent| {
		spawn_local(async move {
			if let Some(deployment_info) = deployment_info.get() {
				delete_deployment(
					access_token.get(),
					Some(deployment_info.deployment.id.to_string()),
					current_workspace_id.get(),
				)
				.await;
			}
		})
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
						DeploymentStatus::Running | DeploymentStatus::Created | DeploymentStatus::Stopped => false,
						_ => true,
				}}
			>
				<Icon
					icon={
						match Status::from_deployment_status(deployment_info.deployment.status.clone()) {
							Status::Running => IconType::PauseCircle,
							_ => IconType::PlayCircle
						}
					}
					size={ Size::ExtraSmall }
					class="mr-xs"
					color={Color::Secondary}
				/>
				{
					let status = Status::from_deployment_status(deployment_info.deployment.status.clone());
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
				<Icon
					icon={IconType::Trash2}
					size=Size::ExtraSmall
					class="mr-xs"
				/>
				"DELETE"
			</button>
		}
		.into_view(),
		None => ().into_view(),
	}
}

#[component]
pub fn ManageDeploymentHeader() -> impl IntoView {
	let deployment_info = expect_context::<DeploymentInfoContext>().0;

	view! {
		<ContainerHead>
			<div class="flex justify-between items-center w-full">
				<div class="flex flex-col items-start justify-between">
					<PageTitleContainer clone:deployment_info>
						<PageTitle icon_position={PageTitleIconPosition::End}>
							"Infrastructure"
						</PageTitle>
						<PageTitle
							to="/deployment"
							icon_position={PageTitleIconPosition::End}
							variant={PageTitleVariant::SubHeading}
						>
							"Deployment"
						</PageTitle>
						{
							let deployment_info = deployment_info;
							move || match deployment_info.get() {
								Some(deployment_info) => view! {
									<PageTitle
										variant={PageTitleVariant::Text}
									>
										{deployment_info.deployment.name.clone()}
									</PageTitle>
								}.into_view(),
								None => view! {}.into_view()
							}
						}
					</PageTitleContainer>
				</div>

				<div class="flex items-center justify-center">
					<StartStopButton />
				</div>
			</div>

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
			]}/>

		</ContainerHead>
	}
}
