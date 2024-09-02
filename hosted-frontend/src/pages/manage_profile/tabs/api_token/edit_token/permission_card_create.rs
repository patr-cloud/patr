use std::collections::BTreeMap;

use leptos::ev::Event;
use models::{api::workspace::Workspace, rbac::WorkspacePermission};

use crate::{pages::manage_profile::tabs::api_token::utils::CreateApiTokenInfo, prelude::*};

#[component]
pub fn CreatePermissionCard(
	/// Additional classes
	#[prop(into, optional)]
	class: MaybeSignal<String>,
	/// The workspace data to show
	#[prop(into)]
	workspace: MaybeSignal<WithId<Workspace>>,
) -> impl IntoView {
	let outer_class = class.with(|cname| {
		format!(
			"w-full text-white flex flex-col items-start justify-start gap-md {}",
			cname
		)
	});
	let api_token = expect_context::<RwSignal<CreateApiTokenInfo>>();

	let store_workspace = store_value(workspace.clone());

	let permissions = Signal::derive({
		let workspace = workspace.clone();
		move || api_token.get().permission.get(&workspace.get().id).cloned()
	});

	let is_admin_checkbox = Signal::derive(move || match permissions.get() {
		Some(permission) => permission.is_super_admin(),
		None => false,
	});

	let on_input_checkbox = {
		let workspace_id = workspace.get().clone().id;
		move |ev: Event| {
			ev.prevent_default();

			api_token.update(|token| {
				let permission_exists_or_super_admin = token.permission.contains_key(&workspace_id) ||
					token
						.permission
						.get(&workspace_id)
						.is_some_and(|permission| permission.is_super_admin());

				if permission_exists_or_super_admin {
					token.permission.insert(
						workspace_id,
						WorkspacePermission::Member {
							permissions: BTreeMap::new(),
						},
					);
				} else {
					token
						.permission
						.insert(workspace_id, WorkspacePermission::SuperAdmin);
				}
			})
		}
	};

	view! {
		<div class={outer_class}>
			<p class="li-diamond">
				<strong class="text-md">{workspace.get().data.name}</strong>
			</p>

			<label class="flex items-center justify-start text-grey cursor-pointer" html_for="super_admin">
				<input
					class="mr-xs"
					type="checkbox"
					name="super_admin"
					on:input={on_input_checkbox}
					prop:checked={is_admin_checkbox}
					value={move || store_workspace.with_value(|workspace| workspace.get().id.to_string()) }
				/>
				"Give"
				<strong class="text-medium text-sm mx-xxs text-white">"Super Admin"</strong>
				"permissions for"
				<strong class="mx-xxs text-sm text-white text-medium">
					{move || store_workspace.with_value(|workspace| workspace.get().name.clone()) }" workspace"
				</strong>
			</label>

			{
				move || if !is_admin_checkbox.get() {
					view! {
						<div class="flex flex-col items-start justify-start w-full gap-xs">
							// <ListPermissions permissions={permissions.get()}/>
							// <ChoosePermission
							// 	workspace_id={workspace.get().id}
							// />
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
