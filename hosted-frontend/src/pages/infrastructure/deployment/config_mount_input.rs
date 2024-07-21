use std::rc::Rc;

use leptos::ev::MouseEvent;
use wasm_bindgen::JsCast;
use web_sys::{File, HtmlInputElement};

use crate::prelude::*;

#[component]
pub fn ConfigMountInput(
	/// Additional class names to apply to the outer div, if any.
	#[prop(into, optional)]
	class: MaybeSignal<String>,
	/// List of all mount file names
	#[prop(into, optional, default = vec![].into())]
	mount_points: MaybeSignal<Vec<String>>,
	/// On Pressing Add Button
	#[prop(into, optional, default = Callback::new(|_| ()))]
	on_add: Callback<(MouseEvent, String, String)>,
	// On Pressing Delete Button
	// #[prop(into, optional, default = Callback::new(|_| ()))]
	// on_delete: Callback<(MouseEvent, String)>,
) -> impl IntoView {
	let outer_div_class = class.with(|cname| format!("flex full-width {}", cname));
	let store_filenames = store_value(mount_points.clone());

	let config_file_path = create_rw_signal("".to_string());
	let config_file = create_rw_signal::<Option<File>>(None);

	view! {
		<div class={outer_div_class}>
			<div class="flex-col-2 fr-fs-ct mb-auto mt-md">
				<label html_for="port" class="fr-fs-ct">
					"Config Mounts"
				</label>
			</div>

			<div class="flex-col-10 fc-fs-fs">
				<Show when={move || mount_points.with(|list| !list.is_empty())}>
					<div class="flex full-width">
						<div class="flex-col-12 fc-fc-fs">
							<For
								each={move || store_filenames.with_value(|list| list.get())}
								key={|state| state.clone()}
								let:filename
							>
								<div class="flex full-width mb-xs">
									<div class="flex-col-5 pr-lg">
										<div class="full-width full-height fr-fs-ct px-xl br-sm bg-secondary-light">
											<span class="ml-md txt-disabled">"/etc/config"</span>
											<span class="txt-of-ellipsis of-hidden w-20">
												{filename}
											</span>
										</div>
									</div>

									<div class="flex-col-6">
										<div class="full-width row-card fr-fs-ct px-xl py-sm br-sm bg-secondary-light">
											<span class="mx-md txt-of-ellipsis of-hidden w-45">
												"/etc/"
											</span>
										</div>
									</div>
								</div>
							</For>
						</div>
					</div>
				</Show>

				<div class="flex full-width">
					<div class="flex-col-5 fc-fs-fs pr-lg gap-xxs">
						<Input
							r#type={InputType::Text}
							id="port"
							class="full-width"
							start_text={Some("/etc/config/".to_string())}
							placeholder="Enter File Path"
							on_input={Box::new(move |ev| {
								ev.prevent_default();
								config_file_path.set(event_target_value(&ev))
							})}
						/>
					</div>

					<div class="flex-col-6 fc-fs-fs gap-xxs">
						<Input
							r#type={InputType::File}
							id="file"
							class="full-width"
							placeholder="No File Selected"
							on_input={Box::new(move |ev| {
								ev.prevent_default();

								// TODO: Remove unsafe unwrap
								let elem = ev.target().unwrap().unchecked_into::<HtmlInputElement>();
								if let Some(files) = elem.files() {
									for i in 0..files.length() {
										let file = files.get(i).unwrap();
										config_file.set(Some(file));
									}
								}
							})}
						/>
					</div>

					<div class="flex-col-1 fr-ct-fs">
						<Link
							style_variant={LinkStyleVariant::Contained}
							class="br-sm p-xs ml-md"
							should_submit=false
							on_click={Rc::new(move |ev| {
								on_add.call((ev.clone(), config_file_path.get(), String::new()))
							})}
						>
							<Icon icon={IconType::Plus} color={Color::Secondary}/>
						</Link>
					</div>
				</div>
			</div>
		</div>
	}
}
