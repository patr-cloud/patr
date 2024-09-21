use ev::{Event, KeyboardEvent};
use html::Input;

use crate::imports::*;

#[component]
pub fn OtpInput(
	/// Additional classes to apply to the outer div, if any.
	#[prop(into, optional)]
	class: MaybeSignal<String>,
	/// OTP Value
	#[prop(into)]
	otp: RwSignal<String>,
	/// The length of the otp input
	#[prop(into, optional, default = 6)]
	length: usize,
	/// On Change Input
	#[prop(into, optional, default = Callback::new(|_| ()).into())]
	on_change: Callback<String>,
) -> impl IntoView {
	let class =
		class.with(|cname| format!("w-full flex items-center justify-center gap-xs {cname}"));

	let input_refs = store_value(
		(0..length)
			.map(|_| create_node_ref::<Input>())
			.collect::<Vec<_>>(),
	);

	let otp_arr = Signal::derive(move || {
		(0..length)
			.map(|i| {
				otp.get()
					.chars()
					.nth(i)
					.map(|char| char.to_string())
					.unwrap_or_default()
			})
			.collect::<Vec<_>>()
	});

	let handle_input = move |_: Event, index: usize| {
		let value = input_refs.with_value(|refs| {
			if let Some(x) = refs.get(index).and_then(|x| x.get()).map(|x| x.value()) {
				x
			} else {
				"".to_owned()
			}
		});

		if value.as_str() == "" {
			return;
		}

		let new_otp = otp_arr
			.get()
			.iter()
			.enumerate()
			.map(|(i, x)| {
				if i == index {
					let y = value
						.chars()
						.last()
						.map(|x| x.to_string())
						.unwrap_or("".to_string());
					logging::log!("{}", y);
					y
				} else {
					x.to_owned()
				}
			})
			.collect::<Vec<_>>()
			.join("");

		on_change.call(new_otp.clone());

		if index < length - 1 {
			input_refs.with_value(|input_refs| {
				if let Some(x) = input_refs
					.clone()
					.get(index + 1)
					.and_then(|node| node.get())
				{
					let _ = x.focus();
				}
			});
		}
	};

	let handle_keyboard_input = move |ev: KeyboardEvent, index: usize| {
		if ev.key().as_str() == "Backspace" {
			let new_otp = otp_arr
				.get()
				.iter()
				.enumerate()
				.map(|(i, x)| {
					if i == index {
						"".to_string()
					} else {
						x.to_owned()
					}
				})
				.collect::<Vec<_>>()
				.join("");

			on_change.call(new_otp.clone());

			if index > 0 {
				input_refs.with_value(|input_refs| {
					if let Some(x) = input_refs
						.clone()
						.get(index - 1)
						.and_then(|node| node.get())
					{
						let _ = x.focus();
					}
				})
			}
		}
	};

	view! {
		<div class={class}>
			{
				move || otp_arr.get().iter().enumerate().map(|(i, c)| {
					let _ref = input_refs.with_value(|refs| refs.get(i).unwrap().to_owned());

					view! {
						<div
							class="full-width fr-ct-ct gap-xs"
						>
							<input
								_ref={_ref}
								class="full-width px-xxs txt-center row-card br-sm txt-white txt-lg outline-primary-focus bg-secondary-light"
								type="number"
								placeholder="0"
								prop:value={c}
								on:keydown={move |ev| handle_keyboard_input(ev, i)}
								on:input={move |ev| handle_input(ev, i)}
							/>
						</div>
					}
				}).collect_view()
			}
		</div>
	}
}
