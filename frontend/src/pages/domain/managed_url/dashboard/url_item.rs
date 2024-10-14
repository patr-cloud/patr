use models::api::workspace::managed_url::*;

use super::ManagedUrlCard;
use crate::prelude::*;

/// A Bunch of Collected URLs under the same Domain
#[component]
pub fn ManagedUrls(
	/// Managed URL Item
	#[prop(into)]
	managed_url: Signal<WithId<ManagedUrl>>,
	/// The class names to add to the outer table row
	#[prop(into, optional)]
	class: MaybeSignal<String>,
) -> impl IntoView {
	let class = move || {
		format!(
			"flex flex-col items-start justify-start bg-secondary-light br-bottom-sm w-full cursor-pointer row-card {}",
			class.get()
		)
	};

	let expanded_urls = create_rw_signal(false);

	let icon_type = move || {
		if expanded_urls.get() {
			IconType::ChevronDown
		} else {
			IconType::ChevronRight
		}
	};

	view! {
		<tr class={class}>
			<td
				on:click={move |_| expanded_urls.update(|val: &mut bool| *val = !*val)}
				class="flex justify-start items-center w-full h-full px-md py-sm rounded-sm gap-sm text-sm"
			>
				<Icon icon={MaybeSignal::derive(icon_type)} size={Size::ExtraExtraSmall} />
				"On Patr"
			</td>

			<Show when={move || expanded_urls.get()}>
				<td class="flex flex-col items-start justify-start w-full px-xl pb-md">
					<table class="flex flex-col items-start justify-start w-full">
						<tbody class="flex flex-col items-start justify-start w-full">
							<ManagedUrlCard managed_url={managed_url} enable_radius_on_top=true />
						</tbody>
					</table>
				</td>
			</Show>
		</tr>
	}
}
