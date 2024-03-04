use crate::{pages::*, prelude::*};

#[component]
pub fn PermisisonCard(
	/// Additional classes
	#[prop(into, optional)]
	class: MaybeSignal<String>,
) -> impl IntoView {
	let outer_class = class.with(|cname| format!("full-width txt-white fc-fs-fs gap-md {}", cname));

	view! {
		<div class=outer_class>
			<p class="li-diamond">
				<strong class="txt-md">"{workspace.name}"</strong>
			</p>

			<label class="fr-fs-ct txt-grey cursor-pointer" html_for="isSuperAdmin">
				<input
					type="checkbox"
					name="super-admin"
					class="mr-xs"
				/>
				"Give"
				<strong class="txt-medium txt-sm mx-xxs txt-white">"Super Admin"</strong>
				"permissions for"
				<strong class="mx-xxs txt-sm txt-white txt-medium">"{workspace.name} workspace"</strong>
			</label>

			<div class="fc-fs-fs full-width gap-xs">
				<PermissionItem />
			</div>
		</div>
	}
}
