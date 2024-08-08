mod head;
mod runner_card;

use codee::string::FromToStringCodec;

pub use self::{head::*, runner_card::*};
use crate::prelude::*;

#[component]
pub fn RunnerDashboard() -> impl IntoView {
	let (access_token, _) = use_cookie::<String, FromToStringCodec>(constants::ACCESS_TOKEN);
	let (current_workspace_id, _) =
		use_cookie::<String, FromToStringCodec>(constants::LAST_USED_WORKSPACE_ID);

	let runners_list = create_resource(
		move || (access_token.get(), current_workspace_id.get()),
		move |(access_token, workspace_id)| async move {
			list_runners(workspace_id, access_token).await
		},
	);

	view! {
		<RunnerDashboardHead />
		<ContainerBody class="p-xs gap-md">
			<DashboardContainer
				gap={Size::Large}
				render_items={
					view! {
						<Transition>
							{
								move || match runners_list.get() {
									Some(Ok(data)) => {
										view! {
											<For
												each={move || data.runners.clone()}
												key={|state| state.id.clone()}
												let:runner
											>
												<RunnerCard runner={runner}/>
											</For>
										}.into_view()
									},
									_ => view! {}.into_view()
								}
							}
						</Transition>
					}.into_view()
				}
			/>
		</ContainerBody>
	}
}
