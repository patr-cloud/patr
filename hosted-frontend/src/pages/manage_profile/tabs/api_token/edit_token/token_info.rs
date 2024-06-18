use models::api::user::UserApiToken;

use crate::{pages::convert_offset_to_date, prelude::*};

#[component]
pub fn TokenInfo(
	/// The Token Info Data
	#[prop(into)]
	token_info: MaybeSignal<WithId<UserApiToken>>,
) -> impl IntoView {
	let nbf_date = convert_offset_to_date(token_info.get().token_nbf.clone());
	let exp_date = convert_offset_to_date(token_info.get().token_exp.clone());

	view! {
		<div class="flex mb-xs full-width mb-md">
			<div class="flex-col-2 fr-fs-fs pt-xs">
				<label html_for="name" class="txt-white txt-sm">
					"Token Name"
				</label>
			</div>

			<div class="flex-col-10 fc-fs-fs pl-xl">
				<Input
					r#type={InputType::Text}
					placeholder="Enter Token Name"
					class="full-width"
					value={token_info.get().name.clone()}
					name="token_name"
					id="token_name"
				/>
			</div>
		</div>

		<div class="flex mb-xs full-width mb-md">
			<div class="flex-col-2 fc-fs-fs pt-xs">
				<label html_for="allowedIps" class="txt-white txt-sm">
					"Allowed IP(s)"
				</label>
				<small class="txt-xxs txt-grey">
					"By default, all IP addresses will be allowed."
				</small>
			</div>
			<div class="flex-col-10 fc-fs-fs pl-xl">
				<Input
					r#type={InputType::Text}
					placeholder="Enter Allowed IP addresses"
					class="full-width"

				/>
			</div>
		</div>

		<div class="flex mb-xs full-width mb-md">
			<div class="flex-col-2 fc-fs-fs pt-xs">
				<label html_for="tokenNbf" class="txt-white txt-sm">
					"Token Validity"
				</label>
				<small class="txt-xxs txt-grey">
					"By default, the token will be valid forever from the date created."
				</small>
			</div>
			<div class="flex-col-10 fr-fs-ct pl-xl">
				<div class="flex-col-1 fr-ct-ct txt-sm">"Valid from"</div>
				<div class="flex-col-5 fr-fs-fs pl-md">
					<Input
						r#type={InputType::Date}
						placeholder="Valid From"
						class="full-width cursor-text"
						value={nbf_date}
						name="token_nbf"
						id="token_nbf"
					/>
				</div>
				<div class="flex-col-1 fr-ct-ct txt-sm">"to"</div>
				<div class="flex-col-5 fr-fs-fs pl-md">
					<Input
						r#type={InputType::Date}
						placeholder="Valid Till"
						class="full-width cursor-text"
						value={exp_date}
						name="token_exp"
						id="token_exp"
					/>
				</div>
			</div>
		</div>
	}
}
