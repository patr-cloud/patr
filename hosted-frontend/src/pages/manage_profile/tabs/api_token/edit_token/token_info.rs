use super::ApiTokenInfo;
use crate::{
	pages::{convert_offset_to_date, convert_string_to_datetime},
	prelude::*,
};

#[component]
pub fn TokenInfo() -> impl IntoView {
	let token_info_signal = expect_context::<ApiTokenInfo>().0;

	move || match token_info_signal.get() {
		Some(token_info) => {
			// let _token_info = token_info.clone();
			let nbf_date = Signal::derive({
				let _token_info = token_info.clone();
				move || convert_offset_to_date(_token_info.token_nbf)
			});
			let exp_date = Signal::derive({
				let _token_info = token_info.clone();
				move || convert_offset_to_date(_token_info.token_exp)
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
								ev.stop_propagation();
								token_info_signal
									.update(|token| {
										if let Some(ref mut token) = token {
											token.data.name = event_target_value(&ev);
										}
									});
							})}
							r#type={InputType::Text}
							placeholder="Enter Token Name"
							class="w-full"
							value={token_info.name.clone()}
							name="token_name"
							id="token_name"
						/>
					</div>
				</div>

				<div class="flex w-full mb-md">
					<div class="flex-2 flex flex-col items-start justify-start pt-xs">
						<label html_for="allowedIps" class="text-white text-sm">
							"Allowed IP(s)"
						</label>
						<small class="text-xxs text-grey">
							"By default, all IP addresses will be allowed."
						</small>
					</div>
					<div class="flex-10 flex flex-col items-start justify-start pl-xl">
						<Input
							r#type={InputType::Text}
							placeholder="Enter Allowed IP addresses"
							class="w-full"
						/>

					</div>
				</div>

				<div class="flex w-full mb-md">
					<div class="flex-2 flex-col items-start justify-start pt-xs">
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
									token_info_signal
										.update(|token| {
											if let Some(ref mut token) = token {
												token.data.token_nbf = convert_string_to_datetime(
														event_target_value(&ev),
													)
													.ok();
											}
										});
								})}
								r#type={InputType::Date}
								placeholder="Valid From"
								class="w-full cursor-text"
								value={nbf_date}
								name="token_nbf"
								id="token_nbf"
							/>
						</div>
						<div class="flex-1 flex items-center justify-center text-sm">"to"</div>
						<div class="flex-5 flex items-start justify-start pl-md">
							<Input
								on_input={Box::new(move |ev| {
									ev.prevent_default();
									token_info_signal
										.update(|token| {
											if let Some(ref mut token) = token {
												token.data.token_exp = convert_string_to_datetime(
														event_target_value(&ev),
													)
													.ok();
											}
										});
								})}
								r#type={InputType::Date}
								placeholder="Valid Till"
								class="w-full cursor-text"
								value={exp_date}
								name="token_exp"
								id="token_exp"
							/>
						</div>
					</div>
				</div>
			}
			.into_view()
		}
		None => view! { <p>"Couldn't Load Resource!"</p> }
		.into_view(),
	}
}
