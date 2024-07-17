mod change_password;
mod details_tab;
mod head;

use models::api::workspace::database::DatabaseEngine;
use utils::FromToStringCodec;

pub use self::{change_password::*, details_tab::*, head::*};
use super::DatabaseTypeCard;
use crate::prelude::*;

#[derive(Params, PartialEq)]
pub struct DatabaseParams {
	pub database_id: Option<String>,
}

#[component]
pub fn ManageDatabase() -> impl IntoView {
	let (access_token, _) = use_cookie::<String, FromToStringCodec>(constants::ACCESS_TOKEN);
	let (current_workspace_id, _) =
		use_cookie::<String, FromToStringCodec>(constants::LAST_USED_WORKSPACE_ID);

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
		move || {
			(
				access_token.get(),
				current_workspace_id.get(),
				database_id.get(),
			)
		},
		move |(access_token, workspace_id, database_id)| async move {
			get_database(access_token, Some(database_id), workspace_id).await
		},
	);

	view! {
		<ContainerMain class="full-width full-height mb-md">
			<ManageDatabaseHeader />

			<ContainerBody class="px-xl py-md gap-md">
				<ManageDatabaseDetailsTab />
				// <Transition>
				// 	{
				// 		move || match database_info.get() {
				// 			Some(Ok(data)) => {
				// 				view! {
				// 					<div class="txt-white full-width fr-fc-fc">
				// 						"something"
				// 					</div>
				// 				}.into_view()
				// 			},
				// 			Some(Err(_)) => view! {
				// 				<div class="txt-white full-width fr-fc-fc">
				// 					"Error loading"
				// 				</div>
				// 			}.into_view(),
				// 			None => view! {
				// 				<div class="txt-white full-width fr-fc-fc">
				// 					"Loading"
				// 				</div>
				// 			}.into_view()
				// 		}
				// 	}
				// </Transition>
			</ContainerBody>
		</ContainerMain>
	}
}
