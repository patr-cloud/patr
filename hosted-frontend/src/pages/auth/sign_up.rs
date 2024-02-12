use std::rc::Rc;

use crate::{pages::*, prelude::*};

#[component]
pub fn SignUpForm() -> impl IntoView {
	let show_coupon = create_rw_signal(false);
	let show_coupon_button = create_rw_signal(true);

	view! {
		<div class="box-onboard txt-white">
			<div class="fr-sb-bl mb-lg full-width">
				<h1 class="txt-primary txt-xl txt-medium">"Sign Up"</h1>

				<div class="txt-white txt-thin fr-fs-fs">
					<p>"Existing User? "</p>
					<Link
						to="/login"
						r#type=Variant::Link
						class="ml-xs"
					>
						"Login"
					</Link>
				</div>
			</div>

			<form class="fc-fs-fs full-width">
				<div class="fr-ct-fs full-width">
					<div class="fc-fs-fs flex-col-6 pr-xxs">
						<Input
							class="py-xs"
							r#type=InputType::Text
							id="firstName"
							placeholder="First Name"
							start_icon=Some(
								IconProps::builder().icon(IconType::User).size(Size::Medium).build(),
							)
						/>
					</div>

					<div class="fc-fs-fs flex-col-6 pl-xxs">
						<Input
							class="py-xs"
							r#type=InputType::Text
							id="lastName"
							placeholder="Last Name"
							start_icon=Some(
								IconProps::builder().icon(IconType::User).size(Size::Medium).build(),
							)
						/>
					</div>
				</div>

				<Input
					class="full-width mt-lg"
					r#type=InputType::Text
					id="username"
					placeholder="User Name"
					required=true
					start_icon=Some(
						IconProps::builder().icon(IconType::User).build()
					)
				/>

				<Input
					class="full-width mt-lg"
					r#type=InputType::Email
					id="email"
					placeholder="proton@gmail.com"
					start_icon=Some(
						IconProps::builder().icon(IconType::Mail).build()
					)
				/>

				<div class="full-width mt-xxs">{
						show_coupon_button.get().then(|| view! {
							<Link
								on_click=Rc::new(move |_| {
									show_coupon.update(|val| *val = !*val)
								})
								class="ml-auto"
							>
								{
									if show_coupon.get() {
										"Cancel"
									} else {
										"Have a Coupon Code?"
									}
								}
							</Link>
						}.into_view())
					}

					{
						move || show_coupon.get().then(|| view! {
							<Input
								id="class"
								placeholder="Coupon Code"
								class="full-width mt-xs"
								start_icon=Some(
									IconProps::builder().icon(IconType::Tag).build()
								)
							/>
						})
				}</div>

				<Input
					r#type=InputType::Password
					id="password"
					placeholder="Password"
					class="full-width mt-xxs"
					start_icon=Some(
						IconProps::builder().icon(IconType::Unlock).size(Size::Small).build()
					)
				/>

				<Input
					r#type=InputType::Password
					id="confirmPassword"
					placeholder="Confirm Password"
					class="full-width mt-lg"
					start_icon=Some(
						IconProps::builder().icon(IconType::Lock).size(Size::Small).build()
					)
				/>

				<div class="fr-fe-ct full-width mt-lg">
					<Link class="btn mr-xs" r#type=Variant::Link>
						"ALREADY HAVE AN OTP"
					</Link>

					<Link should_submit=true style_variant=LinkStyleVariant::Contained>
						"NEXT"
					</Link>
				</div>
			</form>
		</div>
	}
}

#[component]
pub fn SignUpPage() -> impl IntoView {
	view! {
		<PageContainer class="bg-image">
			<SignUpForm />
		</PageContainer>
	}
}
