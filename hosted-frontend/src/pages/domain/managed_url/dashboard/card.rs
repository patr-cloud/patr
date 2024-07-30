use std::rc::Rc;

use models::api::workspace::managed_url::*;

use super::ManageURLForm;
use crate::{pages::*, prelude::*};

#[component]
pub fn ManagedUrlCard(
	/// Whether to have borders rounded on the top.
	#[prop(into, optional, default = false.into())]
	enable_radius_on_top: MaybeSignal<bool>,
	/// The class names to add to the outer table row
	#[prop(into, optional)]
	class: MaybeSignal<String>,
	/// Managed URL Item
	#[prop(into, optional, default = None.into())]
	manage_url: MaybeSignal<Option<WithId<ManagedUrl>>>,
) -> impl IntoView {
	let show_update_url = create_rw_signal(false);

	let class = move || {
		class.with(|classname| {
			if show_update_url.get() {
				format!(
					"w-full cursor-default flex items-center justify-center bd-light br-bottom-sm of-hidden {}",
					if enable_radius_on_top.get() {
						"br-top-sm"
					} else {
						""
					}
				)
			} else {
				format!(
					"w-full bd-light row-card bg-secondary-light cursor-default br-bottom-sm flex items-center justify-center {} {}",
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

	let url_type = create_rw_signal("redirect".to_string());
	let sub_domain = create_rw_signal("@".to_string());
	let domain = create_rw_signal("google.com".to_string());
	let path = create_rw_signal("redirect".to_string());
	let url = create_rw_signal("google.com".to_string());
	let port = create_rw_signal::<u16>(3000);
	let http_only = create_rw_signal(false);
	let permanent_redirect = create_rw_signal(true);

	view! {
		<tr class={class}>
			<Show
				when={move || !show_update_url.get()}
				fallback={move || {
					view! {
						<td class="w-full flex items-center justify-center py-lg px-xl overflow-y-scroll">
							<ManageURLForm
								url_type={url_type}
								domain={domain}
								sub_domain={sub_domain}
								path={path}
								is_create_mode={false}
								show_form={show_update_url}
								url={url}
								port={port}
								http_only={http_only}
								permanent_redirect={permanent_redirect}
							/>
						</td>
					}
				}}
			>

				<td class="flex-col-4 flex items-center justify-center">
					<a href="" target="_blank" rel="noreferrer" class="txt-underline flex items-center justify-center">
						<span class="max-w-[35ch] text-ellipsis overflow-hidden">"https://onpatr.cloud"</span>
						<Icon
							icon={IconType::ExternalLink}
							size={Size::ExtraExtraSmall}
							class="ml-xxs"
						/>
					</a>
				</td>

				<td class="flex-col-1 flex items-center justify-center">"Redirect"</td>

				<td class="flex-col-4 flex items-center justify-center">
					<a href="" target="_blank" rel="noreferrer" class="txt-underline flex items-center justify-center">
						<span class="txt-medium txt-of-ellipsis of-hidden max-w-[35ch]">
							"https://onpatr.cloud"
						</span>
						<Icon
							icon={IconType::ExternalLink}
							size={Size::ExtraExtraSmall}
							class="ml-xxs"
						/>
					</a>
				</td>

				<td class="flex-col-2 flex items-center justify-center"></td>

				<td class="flex-col-1 flex justify-around items-center">
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
