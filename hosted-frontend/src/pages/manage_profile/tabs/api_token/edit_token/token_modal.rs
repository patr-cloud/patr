use leptos_use::{use_clipboard, UseClipboardReturn};

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
	let UseClipboardReturn {
		is_supported,
		copy,
		copied,
		..
	} = use_clipboard();

	view! {
		<Modal color_variant={SecondaryColorVariant::Light}>
			<div class="center-modal text-white text-sm flex flex-col items-start justify-start \
			bg-secondary-light br-sm p-xl show-center-modal">
				<h3 class="text-primary text-lg">
					"API Token is "
					{move || if is_regenerated.get() { "Regenerated" } else { "Generated" }}
				</h3>
				<p class="text-sm text-thin my-md">
					{move || {
						if is_regenerated.get() {
							"Your Token has be regenerated. \
							Copy it now as we don't store API Tokens, and won't be able to show it again."
						} else {
							"Your Token has been generated. \
							Copy it now as we don't store API Tokens, and won't be able to show it again."
						}
					}}
				</p>
				<div class="w-full flex items-start justify-start flex-wrap px-md py-xs bg-secondary-medium br-sm overflow-hidden">
					<p class="break-word">
						{
							let token = token.clone();
							move || token.get()
						}
					</p>

					<Show clone:copy clone:token when={move || is_supported.get()}>
						<button
							on:click={
								let copy = copy.clone();
								let token = token.clone();
								move |_| copy(token.get().as_str())
							}
							class="ml-auto btn-icon"
						>
							<Show
								when={move || copied.get()}
								fallback={|| {
									view! { <Icon icon={IconType::Copy} size={Size::ExtraSmall} /> }
								}}
							>
								<Icon icon={IconType::Check} size={Size::ExtraSmall} />
							</Show>
						</button>
					</Show>
				</div>

				<div class="flex items-center justify-start mt-lg ml-auto">
					<Link r#type={Variant::Link} to="/user/api-tokens" class="btn mr-xs">
						"DONE"
					</Link>
				</div>
			</div>
		</Modal>
	}
}
