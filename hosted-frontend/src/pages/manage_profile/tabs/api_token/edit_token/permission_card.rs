use models::api::workspace::Workspace;

use crate::{
	pages::{ChoosePermission, PermissionItem},
	prelude::*,
};

#[component]
pub fn PermisisonCard(
	/// Additional classes
	#[prop(into, optional)]
	class: MaybeSignal<String>,
	/// The workspace data to show
	#[prop(into)]
	workspace: MaybeSignal<WithId<Workspace>>,
) -> impl IntoView {
	let outer_class = class.with(|cname| format!("full-width txt-white fc-fs-fs gap-md {}", cname));

	let is_admin_checkbox = create_rw_signal(false);

	view! {
		<div class={outer_class}>
			<p class="li-diamond">
				<strong class="txt-md">{workspace.get().data.name}</strong>
			</p>

			<label class="fr-fs-ct txt-grey cursor-pointer" html_for="super-admin">
				<input
					prop:checked={is_admin_checkbox}
					on:input=move |ev| {
						logging::log!("{:#?}", event_target_value(&ev));
						is_admin_checkbox.update(|v| *v = !*v);
					}
					type="checkbox"
					name="super-admin"
					class="mr-xs"
				/>
				"Give"
				<strong class="txt-medium txt-sm mx-xxs txt-white">"Super Admin"</strong>
				"permissions for"
				<strong class="mx-xxs txt-sm txt-white txt-medium">
					{workspace.get().data.name}" workspace"
				</strong>
			</label>

			{
				move || if !is_admin_checkbox.get() {
					view! {
						<div class="fc-fs-fs full-width gap-xs">
							<PermissionItem/>
							<ChoosePermission />
						</div>
					}.into_view()
				} else {
					view! {
						<></>
					}.into_view()
				}
			}
		</div>
	}
}
