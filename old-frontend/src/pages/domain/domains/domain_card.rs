use super::{DomainItemType, DomainNameServerType};
use crate::prelude::*;

#[component]
pub fn DomainCard(
	/// Additional classes to apply to the outer <tr /> if any
	#[prop(into, optional)]
	class: MaybeSignal<String>,
	/// The Domain Data to populate the table with
	domain_item: DomainItemType,
) -> impl IntoView {
	let class = move || {
		class.with(|classname| {
			format!(
				"fr-ct-ct bg-secondary-light full-width bd-light br-bottom-sm row-card px-xl {}",
				classname
			)
		})
	};

	view! {
		<tr class={class}>
			<td class="flex-col-4 fr-ct-ct">{domain_item.name}</td>
			<td class="flex-col-3 fr-ct-ct">
				{if domain_item.name_server == DomainNameServerType::External {
					"External Name Server"
				} else {
					"Patr Name Server"
				}}

			</td>
			<td class="flex-col-3 fr-ct-ct">
				{if domain_item.verified {
					view! { <p class="txt-success">"Verified"</p> }
				} else {
					view! {
						<p class="txt-warning fr-fs-ct">
							"Not Verified"
							<ToolTipContainer
								tooltip_width=25.
								icon_color={Color::Warning}
								color_variant={SecondaryColorVariant::Medium}
							>
								"If this domain stays unverified for 15 days, it will be removed automatically"
							</ToolTipContainer>
						</p>
					}
				}}

			</td>
		</tr>
	}
}
