mod create_database;
mod database_card;
mod database_dashboard;
mod database_head;
mod database_plan_card;
mod database_type_card;
mod manage_database;

pub use self::{
	create_database::*,
	database_card::*,
	database_dashboard::*,
	database_head::*,
	database_plan_card::*,
	database_type_card::*,
	manage_database::*,
};
use crate::prelude::*;

#[component]
pub fn DatabasePage() -> impl IntoView {
	view! {
		<ContainerMain class="full-width full-height my-md">
			<Outlet />
		</ContainerMain>
	}
}
