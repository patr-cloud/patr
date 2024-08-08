use std::{
	collections::{BTreeMap, BTreeSet},
	str::FromStr,
};

use ev::MouseEvent;
use codee::string::FromToStringCodec;
use models::{
	api::{user::UserApiToken, workspace::Workspace},
	rbac::{ResourcePermissionType, ResourceType, WorkspacePermission},
};

use super::ApiTokenInfo;
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
) -> impl IntoView {
	let outer_class = class.with(|cname| {
		format!(
			"w-full text-white flex flex-col items-start justify-start gap-md {}",
			cname
		)
	});
	let api_token = expect_context::<ApiTokenInfo>().0;

	let permissions = Signal::derive({
		let workspace = workspace.clone();
		move || {
			api_token
				.get()
				.unwrap()
				.permissions
				.get(&workspace.get().id)
				.map(|id| id.clone())
		}
	});

	let is_admin_checkbox = Signal::derive(move || match permissions.get() {
		Some(permission) => match permission {
			WorkspacePermission::Member { permissions: _ } => false,
			WorkspacePermission::SuperAdmin => true,
		},
		None => false,
	});

	let on_input_checkbox = {
		let workspace_id = workspace.get().clone().id;
		move |ev| {
			api_token.update(|token| {
				token.as_mut().and_then(|token| {
					let permission_exists = token.data.permissions.contains_key(&workspace_id);

					if permission_exists {
						let _ = token.data.permissions.remove(&workspace_id);
					} else {
						token
							.data
							.permissions
							.insert(workspace_id, WorkspacePermission::SuperAdmin);
					}

					Some(())
				});
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
					prop:checked={is_admin_checkbox}
					on:input={on_input_checkbox}
					type="checkbox"
					name="super_admin"
					value={workspace.get().id.to_string()}
					class="mr-xs"
				/>
				"Give"
				<strong class="text-medium text-sm mx-xxs text-white">"Super Admin"</strong>
				"permissions for"
				<strong class="mx-xxs text-sm text-white text-medium">
					{workspace.get().data.name}" workspace"
				</strong>
			</label>

			{
				move || if !is_admin_checkbox.get() {
					view! {
						<div class="flex flex-col items-start justify-start w-full gap-xs">
							<ListPermissions permissions={permissions.get()}/>
							<ChoosePermission
								workspace_id={workspace.get().id.clone()}
							/>
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
