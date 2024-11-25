use super::ApiTokenInfo;
use crate::{
	pages::{
		convert_offset_to_date,
		convert_string_to_datetime,
		manage_profile::tabs::api_token::utils::CreateApiTokenInfo,
	},
	prelude::*,
};

#[component]
pub fn TokenInfo() -> impl IntoView {
	let token_info_signal = expect_context::<ApiTokenInfo>().0;
	let api_token_changes = expect_context::<RwSignal<CreateApiTokenInfo>>();

	let nbf_data = Signal::derive({
		let token_nbf = if api_token_changes.get().token_nbf.is_some() {
			api_token_changes.get().token_nbf.clone()
		} else {
			token_info_signal
				.get()
				.map(|token| token.token_nbf)
				.unwrap_or(None)
		};

		move || convert_offset_to_date(token_nbf)
	});

	let exp_data = Signal::derive({
		let token_exp = if api_token_changes.get().token_exp.is_some() {
			api_token_changes.get().token_exp.clone()
		} else {
			token_info_signal
				.get()
				.map(|token| token.token_exp)
				.unwrap_or(None)
		};

		move || convert_offset_to_date(token_exp)
	});

	view! {
		<div class="flex w-full mb-md">
			<div class="flex-2 flex items-start justify-start pt-xs">
				<label html_for="name" class="text-white text-sm">
					"Token Name"
				</label>
			</div>

			<div class="flex-10 flex flex-col items-start justify-start pl-xl">
				<Input
					on_input={Box::new(move |ev| {
						ev.prevent_default();
						api_token_changes.update(|token| {
							token.name = Some(event_target_value(&ev));
						});
					})}
					r#type={InputType::Text}
					placeholder="Enter Token Name"
					class="w-full"
					value={Signal::derive(move || {
						api_token_changes
							.get()
							.name
							.clone()
							.unwrap_or(
								token_info_signal
									.get()
									.map(|token| token.name.clone())
									.unwrap_or_default()
								)
					})}
				/>
			</div>
		</div>

		<div class="flex w-full mb-md">
			<div class="flex-2 flex flex-col items-start justify-start pt-xs">
				<label html_for="tokenNbf" class="text-white text-sm">
					"Token Validity"
				</label>
				<small class="text-xxs text-grey">
					"By default, the token will be valid forever from the date created."
				</small>
			</div>
			<div class="flex-10 fr-fs-ct pl-xl">
				<div class="flex-1 flex justify-center items-center text-sm">
					"Valid from"
				</div>
				<div class="flex-5 flex items-start justify-start pl-md">
					<Input
						on_input={Box::new(move |ev| {
							ev.prevent_default();
							api_token_changes
								.update(|token| {
									token.token_nbf = convert_string_to_datetime(
										event_target_value(&ev),
									)
									.ok();
								});
						})}
						r#type={InputType::Date}
						placeholder="Valid From"
						class="w-full cursor-text"
						value={nbf_data}
						name="token_nbf"
						id="token_nbf"
					/>
				</div>
				<div class="flex-1 flex items-center justify-center text-sm">"to"</div>
				<div class="flex-5 flex items-start justify-start pl-md">
					<Input
						on_input={Box::new(move |ev| {
							ev.prevent_default();
							api_token_changes
								.update(|token| {
									token.token_exp = convert_string_to_datetime(
										event_target_value(&ev),
									)
									.ok();
								});
						})}
						r#type={InputType::Date}
						placeholder="Valid Till"
						class="w-full cursor-text"
						value={exp_data}
						name="token_exp"
						id="token_exp"
					/>
				</div>
			</div>
		</div>
	}
}
