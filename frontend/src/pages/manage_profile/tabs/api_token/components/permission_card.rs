use std::collections::BTreeMap;

use leptos::ev::Event;
use models::{api::workspace::Workspace, rbac::WorkspacePermission};

use super::super::{
	components::{ChoosePermission, PermissionItem},
	utils::{ApiTokenInfo, ApiTokenPermissions},
};
use crate::prelude::*;

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
				view! { <PermissionItem permission={permission} /> }
			})
			.collect_view(),
		_ => view! { <></> }.into_view(),
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
) -> impl IntoView {
	let outer_class = class.with(|cname| {
		format!(
			"w-full text-white flex flex-col items-start justify-start gap-md {}",
			cname
		)
	});
	let api_token_permissions = expect_context::<ApiTokenPermissions>().0;

	let store_workspace = store_value(workspace.clone());

	let permissions = Signal::derive({
		let workspace = workspace.clone();
		move || {
			api_token_permissions
				.get()
				.unwrap()
				.get(&workspace.get().id)
				.cloned()
		}
	});

	let is_admin_checkbox = Signal::derive(move || match permissions.get() {
		Some(permission) => permission.is_super_admin(),
		None => false,
	});

	let on_input_checkbox = {
		let workspace_id = workspace.get().clone().id;
		move |ev: Event| {
			ev.prevent_default();

			api_token_permissions.update(|permissions| {
				permissions.as_mut().map(|permissions| {
					let permission_exists = permissions.contains_key(&workspace_id);
					if permission_exists {
						permissions.insert(
							workspace_id,
							WorkspacePermission::Member {
								permissions: BTreeMap::new(),
							},
						);
					} else {
						permissions.insert(workspace_id, WorkspacePermission::SuperAdmin);
					}
				});
			});
		}
	};

	view! {
		<div class={outer_class}>
			<p class="li-diamond">
				<strong class="text-md">{workspace.get().data.name}</strong>
			</p>

			<label
				class="flex items-center justify-start text-grey cursor-pointer"
				html_for="super_admin"
			>
				<input
					class="mr-xs"
					type="checkbox"
					name="super_admin"
					on:input={on_input_checkbox}
					prop:checked={is_admin_checkbox}
					value={move || {
						store_workspace.with_value(|workspace| workspace.get().id.to_string())
					}}
				/>
				"Give"
				<strong class="text-medium text-sm mx-xxs text-white">"Super Admin"</strong>
				"permissions for"
				<strong class="mx-xxs text-sm text-white text-medium">
					{move || store_workspace.with_value(|workspace| workspace.get().name.clone())}
					" workspace"
				</strong>
			</label>

			{move || {
				if !is_admin_checkbox.get() {
					view! {
						<div class="flex flex-col items-start justify-start w-full gap-xs">
							<ListPermissions
								permissions={permissions.get()}
							/>
							<ChoosePermission workspace_id={workspace.get().id} />
						</div>
					}
						.into_view()
				} else {
					view! { <></> }.into_view()
				}
			}}
		</div>
	}
}
