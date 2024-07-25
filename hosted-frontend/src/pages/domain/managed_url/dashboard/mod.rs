mod card;
mod create;
mod head;
mod update;
mod url_item;

use std::{rc::Rc, str::FromStr};

use convert_case::{self, Case, Casing};
use models::api::workspace::managed_url::*;
use strum::VariantNames;

pub use self::{card::*, create::*, head::*, update::*, url_item::*};
use crate::prelude::*;

#[component]
pub fn UrlDashboard() -> impl IntoView {
	let show_create = create_rw_signal(false);

	view! {
		<Show when=move || show_create.get()>
			<CreateManagedUrlDashboard
				show_create={show_create}
			/>
		</Show>
		<UrlDashboardHead
			on_click_create={move |_| {
				show_create.set(true);
			}}
		/>
		<ContainerBody class="px-xl">
			<div class="flex flex-col items-start justify-start w-full h-full px-md py-xl gap-md">
				<TableDashboard
					column_grids={vec![4, 1, 4, 2, 1]}
					headings={vec![
						"Managed URL".into_view(),
						"Type".into_view(),
						"Target".into_view(),
						"".into_view(),
						"".into_view(),
					]}

					render_rows={view! {
						<ManagedUrls class="border-b border-border-color"/>
						<ManagedUrls class="border-b border-border-color"/>
						<ManagedUrls class="border-b border-border-color"/>
					}.into_view()}
				/>
			</div>
		</ContainerBody>
	}
}
