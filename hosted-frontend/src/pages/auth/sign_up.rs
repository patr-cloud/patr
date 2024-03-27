use std::rc::Rc;

use leptos_router::ActionForm;
use models::api::auth::*;

use crate::{pages::*, prelude::*};

#[server(CreateAccount, endpoint = "auth/sign-up")]
async fn sign_up(
	username: String,
	password: String,
	first_name: String,
	last_name: String,
	email: String,
) -> Result<Result<CreateAccountResponse, ErrorType>, ServerFnError> {
	logging::log!("{}, {}", username, password);
	Ok(make_api_call::<CreateAccountRequest>(
		ApiRequest::builder()
			.path(CreateAccountPath)
			.query(())
			.headers(CreateAccountRequestHeaders {
				user_agent: UserAgent::from_static("hyper/0.12.2"),
			})
			.body(CreateAccountRequest {
				username,
				password,
				first_name,
				last_name,
				recovery_method: RecoveryMethod::Email {
					recovery_email: email,
				},
			})
			.build(),
	)
	.await
	.map(|res| res.body))
}

#[component]
pub fn SignUpPage() -> impl IntoView {
	view! {
		<Outlet />
	}
}

#[component]
pub fn SignUpForm() -> impl IntoView {
	let show_coupon = create_rw_signal(false);
	let show_coupon_button = create_rw_signal(true);

	let sign_up_action = create_server_action::<CreateAccount>();
	let response = sign_up_action.value();

	let password_input = create_rw_signal("".to_owned());
	let password_confirm_input = create_rw_signal("".to_owned());
	let passwords_match =
		Signal::derive(move || password_input.get() != password_confirm_input.get());

	create_effect(move |_| {
		if let Some(Ok(resp)) = response.get() {
			logging::log!("{:#?}", resp);
		}
	});

	create_effect(move |_| {
		logging::log!(
			"{:?} {:?}",
			password_input.get(),
			password_confirm_input.get()
		)
	});

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

			<ActionForm action=sign_up_action class="fc-fs-fs full-width">
				<div class="fr-ct-fs full-width">
					<div class="fc-fs-fs flex-col-6 pr-xxs">
						<Input
							class="py-xs"
							r#type=InputType::Text
							id="first_name"
							name="first_name"
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
							id="last_name"
							name="last_name"
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
					name="username"
					placeholder="User Name"
					required=true
					start_icon=Some(
						IconProps::builder().icon(IconType::User).build()
					)
				/>

				<Input
					class="full-width mt-lg"
					r#type=InputType::Email
					name="email"
					id="email"
					placeholder="proton@gmail.com"
					start_icon=Some(
						IconProps::builder().icon(IconType::Mail).build()
					)
				/>
				<div class="full-width mt-xxs">{
						move || show_coupon_button.get().then(|| view! {
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
					name="password"
					placeholder="Password"
					class="full-width mt-xxs"
					value=password_input
					start_icon=Some(
						IconProps::builder().icon(IconType::Unlock).size(Size::Small).build()
					)
					on_input=Box::new(move |ev| {
						password_input.set(event_target_value(&ev));
					})
				/>

				<Input
					r#type=InputType::Password
					id="confirmPassword"
					placeholder="Confirm Password"
					class="full-width mt-lg"
					value=password_confirm_input
					start_icon=Some(
						IconProps::builder().icon(IconType::Lock).size(Size::Small).build()
					)
					on_input=Box::new(move |ev| {
						password_confirm_input.set(event_target_value(&ev));
					})
				/>

				<Show when=move || passwords_match.get()>
					<Alert r#type=AlertType::Error class="mt-xs">"Passwords Don't Match"</Alert>
				</Show>

				<div class="fr-fe-ct full-width mt-lg">
					<Link class="btn mr-xs" r#type=Variant::Link>
						"ALREADY HAVE AN OTP"
					</Link>

					<Link
						disabled=passwords_match.get()
						r#type=Variant::Button
						should_submit=true
						style_variant=LinkStyleVariant::Contained
					>
						"NEXT"
					</Link>
				</div>
			</ActionForm>
		</div>
	}
}
