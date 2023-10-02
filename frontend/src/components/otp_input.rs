use std::rc::Rc;

use wasm_bindgen::JsCast;

use crate::prelude::*;

#[component]
pub fn OtpInput(
	/// ID of the input
	#[prop(into, optional)]
	id: MaybeSignal<String>,
	/// The ref of the input
	#[prop(optional)]
	r#ref: Option<NodeRef<html::Input>>,
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
	/// If the component should automatically call the `on_submit` function
	/// when the last digit is typed
	#[prop(into, optional, default = false)]
	auto_submit: bool,
	/// The submit handler for the input
	#[prop(optional)]
	on_submit: Option<Rc<dyn Fn(String)>>,
	/// Additional class names to apply to the input, if any
	#[prop(into, optional)]
	class: MaybeSignal<String>,
) -> impl IntoView {
	let node_ref = r#ref.unwrap_or_else(|| create_node_ref());

	let refs = store_value(
		(0..length)
			.map(|index| {
				(
					index,
					create_node_ref::<html::Input>(),
					create_rw_signal(None),
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
		refs[last_index].1.on_load(|node| {
			_ = node.focus();
		});
	});

	let value = MaybeSignal::derive(move || {
		refs.with_value(|refs| {
			refs.iter()
				.try_fold(String::new(), |mut acc, (_, _, signal)| {
					acc.push_str(&signal.get()?.to_string());
					Some(acc)
				})
		})
		.unwrap_or_default()
	});

	let handle_key_down = move |index: usize,
	                            signal: RwSignal<Option<u8>>,
	                            on_submit: Option<Rc<dyn Fn(String)>>,
	                            e: ev::KeyboardEvent| match e.code().as_str()
	{
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
				refs.iter()
					.try_fold(String::new(), |mut acc, (_, _, signal)| {
						acc.push_str(&signal.get()?.to_string());
						Some(acc)
					})
			});
			if let Some((value, on_submit)) = value.zip(on_submit) {
				on_submit(value);
			}
		}
		digit if digit.starts_with("Digit") || digit.starts_with("Numpad") => {
			e.prevent_default();
			let number = digit
				.chars()
				.last()
				.and_then(|c| c.to_digit(10))
				.map(|n| n as u8);
			signal.set(number);
			let value = refs.with_value(|refs| {
				next_input(index, refs);
				refs.iter()
					.try_fold(String::new(), |mut acc, (_, _, signal)| {
						acc.push_str(&signal.get()?.to_string());
						Some(acc)
					})
			});

			if auto_submit {
				if let Some((value, on_submit)) = value.zip(on_submit) {
					on_submit(value);
				}
			}
		}
		_ => (),
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

	view! {
		<div
			class=move || format!("full-width fr-ct-ct gap-xs {}", class.get())
		>
		<input
			id=move || id.get()
			ref=node_ref
			prop:value=value.get()
			type="text"
			style="display: none;"
			disabled=true />
		{
			refs
				.with_value(|items| {
					items
						.iter()
						.copied()
						.map(|(index, node_ref, signal)| {
							let on_submit = on_submit.clone();
							view! {
								<input
									ref=node_ref
									type="number"
									on:keydown=move |e| {
										handle_key_down(
											index,
											signal,
											on_submit.clone(),
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

type RefsValue = (usize, NodeRef<html::Input>, RwSignal<Option<u8>>);

fn next_input(index: usize, refs: &[RefsValue]) {
	if index < refs.len() - 1 {
		_ = refs[index + 1].1.get().unwrap().focus();
	}
}

fn prev_input(index: usize, refs: &[RefsValue]) {
	if index > 0 {
		_ = refs[index - 1].1.get().unwrap().focus();
	}
}

fn is_disabled(index: usize, refs: &[RefsValue]) -> bool {
	if let Some(rposition) = refs
		.iter()
		.rposition(|(_, _, signal)| signal.get().is_some())
	{
		index > rposition + 1
	} else {
		index != 0
	}
}
