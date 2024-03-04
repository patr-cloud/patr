use std::rc::Rc;

use crate::prelude::*;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub struct UserApiToken {
	pub name: String,
	pub expiry: String,
	pub created: String,
}

#[component]
pub fn ApiTokenCard(
	/// Additional class names to apply to the row, if any.
	#[prop(into, optional)]
	class: MaybeSignal<String>,
	/// The User API Token
	#[prop(into)]
	token: MaybeSignal<UserApiToken>,
) -> impl IntoView {
	let outer_class = class.with(
        |cname| format!("bg-secondary-light full-width row-card px-xl bd-light cursor-pointer br-bottom-sm fr-ct-ct {}", cname)
    );

	view! {
		<tr
			tab_index=0
			class=outer_class
			aria_label="Select API Token"
		>
			<td class="flex-col-4 fr-ct-ct">{token.get().name}</td>
			<td class="flex-col-4 fr-ct-ct">{token.get().expiry}</td>
			<td class="flex-col-4 fr-ct-ct">{token.get().created}</td>
		</tr>
	}
}
