use models::{api::workspace::Workspace, rbac::WorkspacePermission};

use crate::{
	pages::{ChoosePermission, PermissionItem},
	prelude::*,
};

#[component]
fn ListPermissions(
	/// The Permission Items
	#[prop(into)]
	permissions: MaybeSignal<Option<WorkspacePermission>>,
) -> impl IntoView {
	match permissions.get() {
		Some(WorkspacePermission::Member { permissions }) => permissions
			.into_iter()
			.map(|permission| {
				view! {
					<PermissionItem permission={permission} />
				}
			})
			.collect_view(),
		_ => view! {<></>}.into_view(),
	}
}

#[component]
pub fn PermissionCard(
	/// Additional classes
	#[prop(into, optional)]
	class: MaybeSignal<String>,
	/// The workspace data to show
	#[prop(into)]
	workspace: MaybeSignal<WithId<Workspace>>,
	/// Workpspace Permissions
	#[prop(into, optional)]
	permissions: MaybeSignal<Option<WorkspacePermission>>,
) -> impl IntoView {
	let outer_class = class.with(|cname| format!("full-width txt-white fc-fs-fs gap-md {}", cname));
	let is_admin_checkbox = create_rw_signal(match permissions.get() {
		Some(permission) => match permission {
			WorkspacePermission::Member { permissions: _ } => false,
			WorkspacePermission::SuperAdmin => true,
		},
		None => false,
	});

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
					name="super_admin[]"
					value={workspace.get().id.to_string()}
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
							<ListPermissions permissions={permissions.get()}/>
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
