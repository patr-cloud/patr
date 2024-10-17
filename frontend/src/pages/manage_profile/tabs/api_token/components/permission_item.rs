use models::rbac::ResourcePermissionType;

use crate::prelude::*;

#[component]
pub fn PermissionItem(
	/// Permission Item
	#[prop(into)]
	#[allow(unused)]
	permission: MaybeSignal<(Uuid, ResourcePermissionType)>,
) -> impl IntoView {
	view! {
		<div class="w-full text-grey flex items-start justify-start">
			<Textbox value={view! {
				<strong class="text-white text-bold">"{permission.name} "</strong>
				"permission will be applied to "
				<strong class="text-white text-bold">"{permission.name}"</strong>
			}
				.into_view()} />

			<div class="flex items-center justify-center pl-md">
				<Link class="p-xs">
					<Icon icon={IconType::Trash2} color={Color::Error} />
				</Link>
			</div>
		</div>
	}
}
