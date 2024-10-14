use super::DeploymentInfoContext;
use crate::{pages::*, prelude::*};

#[component]
pub fn ManageDeploymentImageHistory() -> impl IntoView {
	let deployment_info = expect_context::<DeploymentInfoContext>().0;
	let (state, _) = AuthState::load();
	let access_token = move || state.get().get_access_token();
	let current_workspace_id = move || state.get().get_last_used_workspace_id();

	let image_history_list = create_resource(
		move || {
			(
				access_token(),
				current_workspace_id().unwrap(),
				deployment_info.get().map(|x| x.deployment.id).unwrap(),
			)
		},
		move |(access_token, current_workspace_id, deployment_id)| async move {
			get_deployment_image_history(access_token, deployment_id, current_workspace_id).await
		},
	);

	view! {
		<div class="flex flex-col items-start justify-start w-full px-md my-xl mx-auto fit-wide-screen">
			<div class="flex flex-col items-start justify-start w-full h-full gap-sm">
				<Transition>
					{move || match image_history_list.get() {
						Some(Ok(data)) => {
							let history = data.deploys;
							view! {
								<For
									each={move || history.clone()}
									key={|log| log.clone()}
									let:child
								>
									<ImageHistoryCard deploy_history={child.clone()} />
								</For>
							}
								.into_view()
						}
						_ => view! { "loading" }.into_view(),
					}}
				</Transition>
			</div>
		</div>
	}
}
