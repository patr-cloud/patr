use std::rc::Rc;

use convert_case::*;
use ev::MouseEvent;
use models::api::workspace::managed_url::*;

use crate::{pages::*, prelude::*};

/// Indivisual URL Card Item
#[component]
pub fn ManagedUrlCard(
	/// Whether to have borders rounded on the top.
	#[prop(into, optional, default = false.into())]
	enable_radius_on_top: MaybeSignal<bool>,
	/// The class names to add to the outer table row
	#[prop(into, optional)]
	class: MaybeSignal<String>,
	/// Managed URL Item
	#[prop(into)]
	managed_url: MaybeSignal<WithId<ManagedUrl>>,
) -> impl IntoView {
	let show_update_url = create_rw_signal(false);

	let show_delete_dialogue = create_rw_signal(false);

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

	let (state, _) = AuthState::load();
	let access_token = Signal::derive(move || state.get().get_access_token());
	let current_workspace_id = Signal::derive(move || state.get().get_last_used_workspace_id());

	let store_managed_url = store_value(managed_url.clone());
	let domain = create_resource(
		move || {
			(
				access_token.get(),
				current_workspace_id.get(),
				managed_url.get().domain_id,
			)
		},
		move |(access_token, current_workspace_id, domain_id)| async move {
			get_domain(
				access_token,
				current_workspace_id,
				Some(domain_id.to_string()),
			)
			.await
		},
	);

	let managed_url = Signal::derive(move || {
		let domain = domain.get().clone();

		if domain.is_none() {
			"Cannot Load Domain".to_string()
		} else if domain.clone().unwrap().is_err() {
			"Cannot Load Domain".to_string()
		} else {
			domain
				.clone()
				.unwrap()
				.unwrap()
				.workspace_domain
				.domain
				.name
				.clone()
		}
	});

	let managed_url_link = Signal::derive({
		move || {
			let domain = domain.get().clone();

			let domain_name = if domain.is_none() {
				"Cannot Load Domain".to_string()
			} else if domain.clone().unwrap().is_err() {
				"Cannot Load Domain".to_string()
			} else {
				domain
					.clone()
					.unwrap()
					.unwrap()
					.workspace_domain
					.domain
					.name
					.clone()
			};

			store_managed_url.with_value(|managed_url| {
				managed_url.clone().with(|managed_url| {
					if managed_url.sub_domain == "@" {
						domain_name.clone().to_string()
					} else if domain.is_none() {
						domain_name.clone().to_string()
					} else {
						format!("{}.{}", managed_url.sub_domain, domain_name.clone())
					}
				})
			})
		}
	});

	let managed_url_id = Signal::derive(move || {
		store_managed_url.with_value(|managed_url| managed_url.get().id.to_string())
	});

	let on_delete = move |_: MouseEvent| {
		spawn_local(async move {
			_ = delete_managed_url(
				access_token.get_untracked(),
				current_workspace_id.get_untracked(),
				managed_url_id.with_untracked(|id| Some(id.clone())),
			)
			.await;
		})
	};

	view! {
		<tr class={class}>
			<Show
				when={move || !show_update_url.get()}
				fallback={move || {
					view! {
						<UpdateManagedUrl
							show_component={show_update_url}
							managed_url={Signal::derive(move || {
								store_managed_url.with_value(|managed_url| managed_url.get())
							})}
						/>
					}
				}}
			>

				<Transition>
					<td class="flex-col-4 flex items-center justify-center">
						<a
							href=""
							target="_blank"
							rel="noreferrer"
							class="txt-underline flex items-center justify-center"
						>
							<span class="max-w-[35ch] text-ellipsis overflow-hidden">
								{managed_url}
							</span>
							<Icon
								icon={IconType::ExternalLink}
								size={Size::ExtraExtraSmall}
								class="ml-xxs"
							/>
						</a>
					</td>

					<td class="flex-col-1 flex items-center justify-center">
						{store_managed_url
							.with_value(|managed_url| {
								managed_url
									.clone()
									.get()
									.data
									.clone()
									.url_type
									.to_string()
									.to_case(Case::Title)
							})}
					</td>

					<td class="flex-col-4 flex items-center justify-center">
						<a
							href=""
							target="_blank"
							rel="noreferrer"
							class="txt-underline flex items-center justify-center"
						>
							<span class="txt-medium txt-of-ellipsis of-hidden max-w-[35ch]">
								{managed_url_link}
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
						{move || {
							if show_delete_dialogue.get() {
								view! {
									<Link on_click={Rc::new(move |ev: &MouseEvent| {
										on_delete(ev.to_owned());
									})}>
										<p class="text-error">"Delete"</p>
									</Link>
									<Link on_click={Rc::new(move |_| {
										show_delete_dialogue.set(false);
									})}>
										<p class="text-white">"Cancel"</p>
									</Link>
								}
									.into_view()
							} else {
								view! {
									<Link on_click={Rc::new(move |_| {
										show_update_url.update(|val| *val = !*val)
									})}>

										<Icon icon={IconType::Edit} size={Size::ExtraSmall} />
									</Link>
									<Link on_click={Rc::new(move |_| {
										show_delete_dialogue.set(true);
									})}>
										<Icon
											icon={IconType::Trash2}
											size={Size::ExtraSmall}
											color={Color::Error}
										/>
									</Link>
								}
									.into_view()
							}
						}}
					</td>
				</Transition>
			</Show>
		</tr>
	}
}
