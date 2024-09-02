use codee::string::FromToStringCodec;
use models::api::workspace::database::Database;

use crate::{
	pages::{DatabaseCard, DatabaseHead},
	prelude::*,
};

#[component]
pub fn DatabaseDashboard() -> impl IntoView {
	let data = create_rw_signal::<Vec<WithId<Database>>>(vec![]);
	let (access_token, _) = use_cookie::<String, FromToStringCodec>(constants::ACCESS_TOKEN);
	let (current_workspace_id, _) =
		use_cookie::<String, FromToStringCodec>(constants::LAST_USED_WORKSPACE_ID);

	let database_list = create_resource(
		move || (access_token.get(), current_workspace_id.get()),
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
								 {
									move || match database_list.get() {
										Some(resp) => {
											match resp {
												Ok(data) => view! {
													<For
														each={move || data.database.clone()}
														key={|state| state.id}
														let:database_info
													>
														<DatabaseCard database={database_info}/>
													</For>
												}.into_view(),
												Err(_) => view! {
													<div class="full-width txt-white">
														"ERROR LOADING DATABASES"
													</div>
													<a href="/database/some">"here"</a>
												}.into_view()
											}
										},
										_ => view! {
											<div class="full-width txt-white">
												"LOADING DATABASES"
											</div>
										}.into_view()
									}
								}
							</Transition>
						}
					}
				/>
			</ContainerBody>
		</ContainerMain>
	}
}
