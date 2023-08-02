use std::rc::Rc;

use wasm_bindgen::JsCast;

use crate::prelude::*;

#[component]
pub fn OtpInput(
	/// The scope of the component
	cx: Scope,
	/// The ID of the input.
	#[prop(into, optional)]
	id: MaybeSignal<String>,
	/// The ref to forward to the input
	#[prop(into, optional)]
	r#ref: Option<NodeRef<html::AnyElement>>,
	/// The default value of the OTP
	#[prop(into, optional)]
	otp: String,
	/// The number of digits in the OTP input
	#[prop(into, optional, default = 6)]
	length: usize,
	/// Whether the input is disabled.
	#[prop(into, optional)]
	disabled: MaybeSignal<bool>,
	/// The color of the input
	#[prop(into, optional)]
	variant: MaybeSignal<SecondaryColorVariant>,
	/// The submit handler for the input
	#[prop(optional)]
	on_change: Option<Rc<dyn Fn(String)>>,
	/// Additional class names to apply to the input, if any
	#[prop(into, optional)]
	class: MaybeSignal<String>,
) -> impl IntoView {
	let refs = store_value(
		cx,
		(0..length)
			.map(|index| {
				(
					index,
					create_node_ref::<html::Input>(cx),
					create_rw_signal(cx, None),
				)
			})
			.collect::<Vec<_>>(),
	);
	let mut last_index = 0;
	otp.chars()
		.filter_map(|c| c.to_digit(10))
		.map(|n| n as u8)
		.take(length)
		.enumerate()
		.for_each(|(index, digit)| {
			refs.with_value(|refs| {
				refs[index].2.set(Some(digit));
				last_index = index;
			});
		});
	refs.with_value(|refs| {
		refs[last_index].1.on_load(cx, |node| {
			_ = node.focus();
		});
	});

	let handle_key_down = move |index: usize,
	                            signal: RwSignal<Option<u8>>,
	                            on_change: Option<Rc<dyn Fn(String)>>,
	                            e: ev::KeyboardEvent| {
		let refs = refs.clone();
		match e.code().as_str() {
			"Backspace" | "Delete" => {
				e.prevent_default();
				signal.set(None);
				refs.with_value(|refs| {
					prev_input(index, refs);
				});
			}
			"ArrowLeft" => {
				e.prevent_default();
				refs.with_value(|refs| {
					prev_input(index, refs);
				});
			}
			"ArrowRight" => {
				e.prevent_default();
				refs.with_value(|refs| {
					next_input(index, refs);
				});
			}
			"Enter" | "NumpadEnter" => {
				e.prevent_default();
				let value = refs.with_value(|refs| {
					refs.iter().try_fold(
						String::new(),
						|mut acc, (_, _, signal)| {
							acc.push_str(&signal.get()?.to_string());
							Some(acc)
						},
					)
				});
				if let Some((value, on_change)) = value.zip(on_change) {
					on_change(value);
				}
			}
			digit
				if digit.starts_with("Digit") ||
					digit.starts_with("Numpad") =>
			{
				e.prevent_default();
				let number = digit
					.chars()
					.last()
					.and_then(|c| c.to_digit(10))
					.map(|n| n as u8);
				signal.set(number);
				let value = refs.with_value(|refs| {
					next_input(index, refs);
					refs.iter().try_fold(
						String::new(),
						|mut acc, (_, _, signal)| {
							acc.push_str(&signal.get()?.to_string());
							Some(acc)
						},
					)
				});

				if let Some((value, on_change)) = value.zip(on_change) {
					on_change(value);
				}
			}
			_ => (),
		}
	};

	let handle_on_paste = move |e: ev::Event| {
		e.prevent_default();
		let Some(data) = e
			.unchecked_into::<web_sys::ClipboardEvent>()
			.clipboard_data()
			.and_then(|data| data.get_data("Text").ok())
		else {
			return;
		};
		let mut last_index = 0;
		data.chars()
			.filter_map(|c| c.to_digit(10))
			.map(|n| n as u8)
			.take(length)
			.enumerate()
			.for_each(|(index, digit)| {
				refs.with_value(|refs| {
					refs[index].2.set(Some(digit));
					last_index = index;
				});
			});
		refs.with_value(|refs| {
			_ = refs[last_index].1.get().unwrap().focus();
		});
	};

	view! { cx,
		<div
			class=move || format!("full-width fr-ct-ct gap-xs {}", class.get())
		>
		{
			refs
				.with_value(|items| {
					items
						.iter()
						.copied()
						.map(|(index, node_ref, signal)| {
							let on_change = on_change.clone();
							view! { cx,
								<input
									ref=node_ref
									type="number"
									on:keydown=move |e| {
										handle_key_down(
											index,
											signal,
											on_change.clone(),
											e
										);
									}
									on:paste=handle_on_paste
									prop:value=move || signal.get()
									placeholder="0"
									disabled=move || {
										disabled.get() || {
											refs.with_value(|refs| {
												is_disabled(index, refs)
											})
										}
									}
									inputmode="numeric"
									style="color: transparent; text-shadow: 0 0 0 white;"
									class=move || format!(
										concat!(
											"full-width px-xxs txt-center row-card ",
											"br-sm txt-white txt-lg outline-primary-focus ",
											"bg-secondary-{}"
										),
										variant.get()
									)
								/>
							}
						})
						.collect::<Vec<_>>()
				})
		}
		</div>
	}
}

fn next_input(
	index: usize,
	refs: &Vec<(usize, NodeRef<html::Input>, RwSignal<Option<u8>>)>,
) {
	if index < refs.len() - 1 {
		_ = refs[index + 1].1.get().unwrap().focus();
	}
}

fn prev_input(
	index: usize,
	refs: &Vec<(usize, NodeRef<html::Input>, RwSignal<Option<u8>>)>,
) {
	if index > 0 {
		_ = refs[index - 1].1.get().unwrap().focus();
	}
}

fn is_disabled(
	index: usize,
	refs: &Vec<(usize, NodeRef<html::Input>, RwSignal<Option<u8>>)>,
) -> bool {
	if let Some(rposition) = refs
		.iter()
		.rposition(|(_, _, signal)| signal.get().is_some())
	{
		index > rposition + 1
	} else {
		index != 0
	}
}
