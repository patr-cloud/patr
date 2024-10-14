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
			<div class="flex-2 flex justify-start items-center mb-auto mt-md">
				<label html_for="port" class="flex justify-start items-center">
					"Config Mounts"
				</label>
			</div>

			<div class="flex-10 flex flex-col items-start justify-start">
				<Show when={move || mount_points.with(|list| !list.is_empty())}>
					<div class="flex full-width">
						<div class="flex-12 flex flex-col items-center justify-center">
							<For
								each={move || store_filenames.with_value(|list| list.get())}
								key={|state| state.clone()}
								let:filename
							>
								<div class="flex w-full mb-xs">
									<div class="flex-5 pr-lg">
										<div class="w-full h-full flex justify-start items-center px-xl br-sm bg-secondary-light">
											<span class="ml-md text-disabled">"/etc/config"</span>
											<span class="text-ellipsis overflow-hidden w-[20ch]">
												{filename}
											</span>
										</div>
									</div>

									<div class="flex-6">
										<div class="w-full row-card flex justify-start items-center px-xl py-sm br-sm bg-secondary-light">
											<span class="mx-md text-ellipsis overflow-hidden w-[45ch]">
												"/etc/"
											</span>
										</div>
									</div>
								</div>
							</For>
						</div>
					</div>
				</Show>

				<form class="flex w-full">
					<div class="flex-5 flex flex-col items-start justify-start pr-lg gap-xxs">
						<Input
							r#type={InputType::Text}
							id="port"
							class="w-full"
							start_text={Some("/etc/config/".to_string())}
							placeholder="Enter File Path"
							on_input={Box::new(move |ev| {
								ev.prevent_default();
								config_file_path.set(event_target_value(&ev))
							})}
						/>
					</div>

					<div class="flex-6 flex flex-col items-start justify-start gap-xxs">
						<Input
							r#type={InputType::File}
							id="file"
							class="w-full"
							placeholder="No File Selected"
							on_input={Box::new(move |ev| {
								ev.prevent_default();
								let elem = ev
									.target()
									.unwrap()
									.unchecked_into::<HtmlInputElement>();
								if let Some(files) = elem.files() {
									for i in 0..files.length() {
										let file = files.get(i).unwrap();
										config_file.set(Some(file));
									}
								}
							})}
						/>
					</div>

					<div class="flex-1 flex justify-center items-start">
						<Link
							style_variant={LinkStyleVariant::Contained}
							class="br-sm p-xs ml-md"
							should_submit=false
							on_click={Rc::new(move |ev| {
								on_add.call((ev.clone(), config_file_path.get(), String::new()))
							})}
						>
							<Icon icon={IconType::Plus} color={Color::Secondary} />
						</Link>
					</div>
				</form>
			</div>
		</div>
	}
}
