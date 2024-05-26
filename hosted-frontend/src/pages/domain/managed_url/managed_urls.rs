use crate::{pages::ManagedUrlCard, prelude::*};

#[component]
pub fn ManagedUrls(
	/// The class names to add to the outer table row
	#[prop(into, optional)]
	class: MaybeSignal<String>,
) -> impl IntoView {
	let class = move || {
		format!(
			"fc-fs-fs bg-secondary-light br-bottom-sm full-width cursor-pointer row-card {}",
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
		<tr class={class} on:click={move |_| expanded_urls.update(|val: &mut bool| *val = !*val)}>
			<td class="fr-fs-ct full-width px-md py-sm full-height br-sm gap-sm txt-sm">
				<Icon icon={MaybeSignal::derive(icon_type)} size={Size::ExtraExtraSmall}/>
				"On Patr"
			</td>

			<Show when={move || expanded_urls.get()}>
				<td class="full-width px-xl fc-fs-fs pb-md">
					<table class="fc-fs-fs full-width">
						<tbody class="fc-fs-fs full-width">
							<ManagedUrlCard enable_radius_on_top=true/>
							<ManagedUrlCard enable_radius_on_top=true/>
						</tbody>
					</table>
				</td>
			</Show>
		</tr>
	}
}
