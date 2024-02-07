use leptos::html::Col;

use crate::prelude::*;

#[component]
pub fn ManagedUrlCard(
	/// Whether to have borders rounded on the top.
	#[prop(into, optional, default = false.into())]
	enable_radius_on_top: MaybeSignal<bool>,
	/// The class names to add to the outer table row
	#[prop(into, optional)]
	class: MaybeSignal<String>,
) -> impl IntoView {
	let class = move || {
		class.with(|classname| {
			format!(
				"full-width bd-light row-card bg-secondary-light cursor-default br-bottom-sm fr-ct-ct {} {}",
				classname,
				if enable_radius_on_top.get() {
					"br-top-sm"
				} else {
					""
				}
			)
		})
	};

	view! {
		<>
			<tr class=class>
				<td class="flex-col-4 fr-ct-ct">
					<a
						href=""
						target="_blank"
						rel="noreferrer"
						class="txt-underline fr-ct-ct"
					>
						<span class="w-35 txt-of-ellipsis of-hidden">
							"https://onpatr.cloud"
						</span>
						<Icon
							icon=IconType::ExternalLink
							size=Size::ExtraExtraSmall
							class="ml-xxs"
						/>
					</a>
				</td>

				<td class="flex-col-1 fr-ct-ct">
					"Redirect"
				</td>

				<td class="flex-col-4 fr-ct-ct">
					<a
						href=""
						target="_blank"
						rel="noreferrer"
						class="txt-underline fr-ct-ct"
					>
						<span class="txt-medium txt-of-ellipsis of-hidden w-35">
							"https://onpatr.cloud"
						</span>
						<Icon
							icon=IconType::ExternalLink
							size=Size::ExtraExtraSmall
							class="ml-xxs"
						/>
					</a>
				</td>

				<td class="flex-col-2 fr-ct-ct">
				</td>

				<td class="flex-col-1 fr-sa-ct">
					<button>
						<Icon icon=IconType::Edit size=Size::ExtraSmall />
					</button>
					<button>
						<Icon icon=IconType::Trash2 size=Size::ExtraSmall color=Color::Error />
					</button>
				</td>
			</tr>
		</>
	}
}
