mod change_password;
mod details_tab;
mod head;

pub use self::{change_password::*, details_tab::*, head::*};
use crate::prelude::*;

#[derive(Params, PartialEq)]
pub struct DatabaseParams {
	pub database_id: Option<String>,
}

#[component]
pub fn ManageDatabase() -> impl IntoView {
	let (state, _) = AuthState::load();
	let access_token = move || state.get().get_access_token();
	let current_workspace_id = move || state.get().get_last_used_workspace_id();

	let params = use_params::<DatabaseParams>();
	let database_id = Signal::derive(move || {
		params.with(|params| {
			params
				.as_ref()
				.map(|param| param.database_id.clone().unwrap_or_default())
				.unwrap_or_default()
		})
	});

	let database_info = create_resource(
		move || (access_token(), current_workspace_id(), database_id.get()),
		move |(access_token, workspace_id, database_id)| async move {
			get_database(access_token, Some(database_id), workspace_id).await
		},
	);

	view! {
		<ContainerMain class="full-width full-height mb-md">
			<Transition>
				{move || match database_info.get() {
					Some(Ok(database)) => {
						let id = database.database.id;
						let name = database.database.name.clone();
						view! {
							<ManageDatabaseHeader
								id={Signal::derive(move || Some(id))}
								name={Signal::derive(move || name.clone())}
							/>
							<ContainerBody class="px-xl py-md gap-md">
								<ManageDatabaseDetailsTab database_info={Signal::derive(move || {
									database.database.clone()
								})} />
							</ContainerBody>
						}
							.into_view()
					}
					Some(Err(_)) => {
						view! {
							<ManageDatabaseHeader />
							<ContainerBody class="px-xl py-md gap-md">
								<div class="txt-white full-width fr-fc-fc">"Error loading"</div>
							</ContainerBody>
						}
							.into_view()
					}
					None => {
						view! {
							<ManageDatabaseHeader />
							<ContainerBody class="px-xl py-md gap-md">
								<div class="txt-white full-width fr-fc-fc">"Loading"</div>
							</ContainerBody>
						}
							.into_view()
					}
				}}
			</Transition>
		</ContainerMain>
	}
}
