use crate::prelude::*;

#[component]
pub fn TokenModal(
	/// Whether the API Token is regenerated, or freshly created
	#[prop(into)]
	is_regenerated: MaybeSignal<bool>,
	/// The Generated Token
	#[prop(into)]
	token: MaybeSignal<String>,
) -> impl IntoView {
	view! {
		<Modal
			color_variant={SecondaryColorVariant::Light}
		>
			<div
				class="center-modal txt-white txt-sm fc-fs-fs bg-secondary-light br-sm p-xl show-center-modal"
			>
				<h3 class="txt-primary txt-lg">
					"API Token is "
					{move || if is_regenerated.get() {"Regenerated"} else {"Generated"}}
				</h3>
				<p class="txt-sm txt-thin my-md">
					{
						move || if is_regenerated.get() {
							"Your Token has be regenerated. \
							Copy it now as we don't store API Tokens, and won't be able to show it again."
						} else {
							"Your Token has been generated. \
							Copy it now as we don't store API Tokens, and won't be able to show it again."
						}
					}
				</p>
				<div class="full-width fr-fs-fs f-wrap px-md py-xs bg-secondary-medium br-sm of-hidden">
					<p class="break-word">{
						let token = token.clone();
						move || token.get()
					}</p>
					<button
						class="ml-auto btn-icon"
					>
						<Icon icon={IconType::Copy} size=Size::ExtraSmall />
					</button>
				</div>

				<div class="fr-fs-ct mt-lg ml-auto">
					<Link r#type={Variant::Link} to="/user/api-tokens" class="btn mr-xs">
						"DONE"
					</Link>
				</div>
			</div>
		</Modal>
	}
}
