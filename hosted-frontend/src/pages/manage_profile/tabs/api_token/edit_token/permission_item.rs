use crate::prelude::*;

#[component]
pub fn PermissionItem() -> impl IntoView {
	view! {
		<div class="full-width txt-grey fr-fs-fs">
			<Textbox
				value={
					view! {
						<strong class="txt-white txt-bold">"{permission.name} "</strong>
						"permission will be applied to "
						<strong class="txt-white txt-bold">"{permission.name}"</strong>
					}.into_view()
				}
			/>

			<div class="fr-ct-ct pl-md">
				<Link class="p-xs">
					<Icon icon={IconType::Trash2} color={Color::Error}/>
				</Link>
			</div>
		</div>
	}
}
