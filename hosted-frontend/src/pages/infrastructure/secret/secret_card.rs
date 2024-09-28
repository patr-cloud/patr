use super::SecretListItem;
use crate::prelude::*;

#[component]
pub fn SecretCard(
	/// The Secret Table Row Data to populate the Table
	secret_item: SecretListItem,
	/// The class names to add to the outer table row
	#[prop(into, optional)]
	class: MaybeSignal<String>,
) -> impl IntoView {
	let class = move || {
		format!(
			"bg-secondary-light full-width fr-ct-ct px-xl bd-light br-smooth-sm row-card {}",
			class.get()
		)
	};
	view! {
		<tr class={class}>
			<td class="flex-col-11 fr-fs-ct">{secret_item.name}</td>
			<td class="flex-col-1 fr-sa-ct">
				<button>
					<Icon icon={IconType::Edit} size={Size::ExtraSmall} />
				</button>
				<button>
					<Icon icon={IconType::Trash2} size={Size::ExtraSmall} color={Color::Error} />
				</button>
			</td>
		</tr>
	}
}
