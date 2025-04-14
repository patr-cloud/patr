use crate::{
	pages::{DatabaseCard, DatabaseHead},
	prelude::*,
};

#[component]
pub fn DatabaseDashboard() -> impl IntoView {
	let (state, _) = AuthState::load();
	let access_token = move || state.get().get_access_token();
	let current_workspace_id = move || state.get().get_last_used_workspace_id();

	let database_list = create_resource(
		move || (access_token(), current_workspace_id()),
		move |(access_token, workspace_id)| async move {
			list_database(access_token, workspace_id).await
		},
	);

	view! {
		<ContainerMain class="full-width full-height mb-md">
			<DatabaseHead />
			<ContainerBody>
				<DashboardContainer
					gap={Size::Large}
					render_items={
						view! {
							<Transition>
								{move || match database_list.get() {
									Some(resp) => {
										match resp {
											Ok(data) => {
												view! {
													<For
														each={move || data.database.clone()}
														key={|state| state.id}
														let:database_info
													>
														<DatabaseCard database={database_info} />
													</For>
												}
													.into_view()
											}
											Err(_) => {
												view! {
													<div class="full-width txt-white">
														"ERROR LOADING DATABASES"
													</div>
													<a href="/database/some">"here"</a>
												}
													.into_view()
											}
										}
									}
									_ => {
										view! {
											<div class="full-width txt-white">"LOADING DATABASES"</div>
										}
											.into_view()
									}
								}}
							</Transition>
						}
					}
				/>
			</ContainerBody>
		</ContainerMain>
	}
}
