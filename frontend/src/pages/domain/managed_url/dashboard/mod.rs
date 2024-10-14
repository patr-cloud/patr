mod card;
mod create;
mod head;
mod update;
mod url_form;
mod url_item;

pub use self::{card::*, create::*, head::*, update::*, url_form::*, url_item::*};
use crate::prelude::*;

#[component]
pub fn UrlDashboard() -> impl IntoView {
	let show_create = create_rw_signal(false);

	let (state, _) = AuthState::load();
	let access_token = Signal::derive(move || state.get().get_access_token());
	let current_workspace_id = Signal::derive(move || state.get().get_last_used_workspace_id());

	let managed_url_list = create_resource(
		move || (access_token.get(), current_workspace_id.get()),
		move |(access_token, workspace_id)| async move {
			list_managed_urls(workspace_id, access_token).await
		},
	);

	let table_rows = move || match managed_url_list.get() {
		Some(Ok(data)) => {
			// let x = data.urls.get(0).expect("").data;
			logging::log!(
				"delete this line frontend/src/pages/domain/managed_url/dashboard/mod.rs:35"
			);
			view! {
				<For each={move || data.urls.clone()} key={|state| state.id} let:url>
					<ManagedUrls
						managed_url={Signal::derive(move || url.clone())}
						class="border-b border-border-color"
					/>
				</For>
			}
		}
		.into_view(),
		_ => view! {}.into_view(),
	};

	view! {
		<Show when={move || show_create.get()}>
			<CreateManagedUrlDashboard show_create={show_create} />
		</Show>
		<UrlDashboardHead on_click_create={move |_| {
			show_create.set(true);
		}} />
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

					render_rows={view! { <Transition>{table_rows}</Transition> }.into_view()}
				/>
			</div>
		</ContainerBody>
	}
}
