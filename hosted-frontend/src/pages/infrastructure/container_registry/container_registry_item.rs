use leptos_router::A;

use super::ContainerRegistryItem;
use crate::prelude::*;

#[component]
pub fn ContainerRegistryCard(item: ContainerRegistryItem) -> impl IntoView {
	let stored_item = store_value(item);

	view! {
		<tr
			tab_index=0
			class="fr-ct-ct full-width row-card px-xl br-bottom-sm bd-light bg-secondary-light txt-white"
		>
			<td class="flex-col-5 fr-ct-ct">
				<span class="of-hidden txt-of-ellipsis w-35">{item.name}</span>
			</td>
			<td class="flex-col-2 fr-ct-ct">{item.size}</td>
			<td class="flex-col-4 fr-ct-ct">{item.date_created}</td>
			<td class="flex-col-1 fr-fe-ct">
				<button class="btn-icon">
					<Icon icon=IconType::Trash2 size=Size::ExtraSmall color=Color::Error/>
				</button>
			</td>
		</tr>
	}
}
