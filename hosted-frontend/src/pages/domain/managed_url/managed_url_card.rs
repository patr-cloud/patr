use std::rc::Rc;

use crate::{pages::*, prelude::*};

#[component]
pub fn ManagedUrlCard(
	/// Whether to have borders rounded on the top.
	#[prop(into, optional, default = false.into())]
	enable_radius_on_top: MaybeSignal<bool>,
	/// The class names to add to the outer table row
	#[prop(into, optional)]
	class: MaybeSignal<String>,
) -> impl IntoView {
	let show_update_url = create_rw_signal(false);

	let class = move || {
		class.with(|classname| {
			if show_update_url.get() {
				format!(
					"full-width cursor-default fr-ct-ct bd-light br-bottom-sm of-hidden {}",
					if enable_radius_on_top.get() {
						"br-top-sm"
					} else {
						""
					}
				)
			} else {
				format!(
					"full-width bd-light row-card bg-secondary-light cursor-default br-bottom-sm fr-ct-ct {} {}",
					classname,
					if enable_radius_on_top.get() {
						"br-top-sm"
					} else {
						""
					}
				)
			}
		})
	};

	view! {
		<tr class={class}>
			<Show
				when={move || !show_update_url.get()}
				fallback={move || {
					view! {
						<td class="full-width fr-ct-ct">
							<UpdateManagedUrl show_update_component={show_update_url}/>
						</td>
					}
				}}
			>

				<td class="flex-col-4 fr-ct-ct">
					<a href="" target="_blank" rel="noreferrer" class="txt-underline fr-ct-ct">
						<span class="w-35 txt-of-ellipsis of-hidden">"https://onpatr.cloud"</span>
						<Icon
							icon={IconType::ExternalLink}
							size={Size::ExtraExtraSmall}
							class="ml-xxs"
						/>
					</a>
				</td>

				<td class="flex-col-1 fr-ct-ct">"Redirect"</td>

				<td class="flex-col-4 fr-ct-ct">
					<a href="" target="_blank" rel="noreferrer" class="txt-underline fr-ct-ct">
						<span class="txt-medium txt-of-ellipsis of-hidden w-35">
							"https://onpatr.cloud"
						</span>
						<Icon
							icon={IconType::ExternalLink}
							size={Size::ExtraExtraSmall}
							class="ml-xxs"
						/>
					</a>
				</td>

				<td class="flex-col-2 fr-ct-ct"></td>

				<td class="flex-col-1 fr-sa-ct">
					<Link on_click={Rc::new(move |_| {
						show_update_url.update(|val| *val = !*val)
					})}>

						<Icon icon={IconType::Edit} size={Size::ExtraSmall}/>
					</Link>
					<Link>
						<Icon icon={IconType::Trash2} size={Size::ExtraSmall} color={Color::Error}/>
					</Link>
				</td>
			</Show>
		</tr>
	}
}
