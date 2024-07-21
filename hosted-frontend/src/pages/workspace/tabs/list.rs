use utils::FromToStringCodec;

use crate::prelude::*;

#[component]
pub fn ListWorksapce() -> impl IntoView {
	let (access_token, _) = use_cookie::<String, FromToStringCodec>(constants::ACCESS_TOKEN);

	let workspace_list = create_resource(
		move || access_token.get(),
		move |value| async move { list_user_workspace(value).await },
	);

	view! {
		<div class="fc-fs-fs full-width full-height fit-wide-screen mx-auto px-md my-xl">
			<h4 class="txt-white txt-lg pb-md txt-white">"List of Workspaces"</h4>

			<ul>
				<Transition>
					{
						move || match workspace_list.get() {
							Some(Ok(workspace_list)) => {
								view! {
									<For
										each={move || workspace_list.workspaces.clone()}
										key={|state| state.id.clone()}
										let:workspace
									>
										<li class="li-diamond fr-ct-ct gap-md">
											<p class="txt-white px-xl py-sm br-sm w-50 bg-secondary-light full-width">
												{workspace.name.clone()}
											</p>
										</li>
									</For>
								}.into_view()
							},
							_ => view! {}.into_view()
						}
					}
				</Transition>
			</ul>
		</div>
	}
}
