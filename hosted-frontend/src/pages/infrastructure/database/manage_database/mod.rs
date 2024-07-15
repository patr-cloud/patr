mod details_tab;
mod head;

pub use self::{details_tab::*, head::*};
use crate::prelude::*;

#[component]
pub fn ManageDatabase() -> impl IntoView {
	view! {
		<ContainerMain class="full-width full-height mb-md">
			<ManageDatabaseHeader />

			<ContainerBody class="px-xxl py-xl gap-md">
				<ManageDatabaseDetailsTab/>
			</ContainerBody>
		</ContainerMain>
	}
}
