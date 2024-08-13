use std::rc::Rc;

use ev::SubmitEvent;
use models::api::auth::*;

use crate::prelude::*;

#[component]
pub fn SignUpForm() -> impl IntoView {
	let show_coupon = create_rw_signal(false);
	let show_coupon_button = create_rw_signal(true);

	let loading = create_rw_signal(false);

	// let response = sign_up_action.value();

	let first_name = create_rw_signal("".to_owned());
	let last_name = create_rw_signal("".to_owned());
	let email_input = create_rw_signal("".to_owned());
	let username_input = create_rw_signal("".to_owned());
	let password_input = create_rw_signal("".to_owned());
	let password_confirm_input = create_rw_signal("".to_owned());
	let passwords_match =
		Signal::derive(move || password_input.get() != password_confirm_input.get());

	let name_error = create_rw_signal("".to_owned());
	let username_error = create_rw_signal("".to_owned());
	let email_error = create_rw_signal("".to_owned());
	let password_confirm_error = create_rw_signal("".to_owned());
	let password_error = create_rw_signal("".to_owned());

	let handle_errors = move |error| match error {
		ServerFnError::WrappedServerError(ErrorType::UsernameUnavailable) => {
			username_error.set("Username Not Available".to_owned());
		}
		ServerFnError::WrappedServerError(ErrorType::EmailUnavailable) => {
			email_error.set("Email Not Available".to_owned());
		}
		e => {
			password_error.set(e.to_string());
		}
	};

	let on_submit_sign_up = move |ev: SubmitEvent| {
		ev.prevent_default();
		loading.set(true);

		name_error.set("".to_string());
		password_error.set("".to_string());
		username_error.set("".to_string());
		password_confirm_error.set("".to_string());

		if first_name.get().is_empty() || last_name.get().is_empty() {
			name_error.set("Please Give us a Name".to_string());
			loading.set(false);
			return;
		}

		if username_input.get().is_empty() {
			username_error.set("Give Username".to_string());
			loading.set(false);
			return;
		}

		if password_input.get().is_empty() {
			password_error.set("Give Password".to_string());
			loading.set(false);
			return;
		}

		if password_confirm_input.get().is_empty() {
			password_confirm_error.set("Please Re-Enter Password".to_string());
			loading.set(false);
			return;
		}

		spawn_local(async move {
			let response = sign_up(
				username_input.get_untracked(),
				password_input.get_untracked(),
				first_name.get_untracked(),
				last_name.get_untracked(),
				email_input.get_untracked(),
			)
			.await;

			match response {
				Ok(CreateAccountResponse {}) => {}
				Err(err) => {
					handle_errors(err);
				}
			}

			loading.set(false);
		})
	};

	// create_effect(move |_| {
	// 	if let Some(resp) = response.get() {
	// 		logging::log!("{:#?}", resp);
	// 		match resp {
	// 			Ok(CreateAccountResponse {}) => {}
	// 			Err(err) => {
	// 				logging::log!("{:#?}", err);
	// 				handle_errors(err);
	// 			}
	// 		}
	// 	}
	// });

	view! {
		<div class="box-onboard text-white">
			<div class="flex justify-between items-baseline mb-lg w-full">
				<h1 class="text-primary text-xl text-medium">"Sign Up"</h1>

				<div class="text-white text-thin flex justify-start items-start">
					<p>"Existing User? "</p>
					<Link to="/login" r#type={Variant::Link} class="ml-xs">
						"Login"
					</Link>
				</div>
			</div>

			<form on:submit={on_submit_sign_up} class="flex flex-col items-start justify-start w-full">
				<div class="flex justify-center items-start w-full">
					<div class="flex flex-col items-start justify-start flex-col-6 pr-xxs">
						<Input
							class="py-xs"
							r#type={InputType::Text}
							id="first_name"
							name="first_name"
							placeholder="First Name"
							value={first_name}
							on_input={Box::new(move |ev| {
								first_name.set(event_target_value(&ev))
							})}
							start_icon={Some(
								IconProps::builder().icon(IconType::User).size(Size::Medium).build(),
							)}
						/>
					</div>

					<div class="flex flex-col items-start justify-start flex-col-6 pl-xxs">
						<Input
							class="py-xs"
							r#type={InputType::Text}
							id="last_name"
							name="last_name"
							placeholder="Last Name"
							value={last_name}
							on_input={Box::new(move |ev| {
								last_name.set(event_target_value(&ev))
							})}
							start_icon={Some(
								IconProps::builder().icon(IconType::User).size(Size::Medium).build(),
							)}
						/>
					</div>
				</div>
				<Show when={move || !name_error.get().is_empty()}>
					<Alert r#type={AlertType::Error} class="mt-xs">
						{move || name_error.get()}
					</Alert>
				</Show>

				<Input
					class="w-full mt-lg"
					r#type={InputType::Text}
					id="username"
					name="username"
					placeholder="User Name"
					start_icon={Some(IconProps::builder().icon(IconType::User).build())}
					value={username_input}
					on_input={Box::new(move |ev| {
						username_input.set(event_target_value(&ev))
					})}
				/>

				<Show when={move || !username_error.get().is_empty()}>
					<Alert r#type={AlertType::Error} class="mt-xs">
						{move || username_error.get()}
					</Alert>
				</Show>

				<Input
					class="w-full mt-lg"
					r#type={InputType::Email}
					name="email"
					id="email"
					placeholder="proton@gmail.com"
					start_icon={Some(IconProps::builder().icon(IconType::Mail).build())}
					value={email_input}
					on_input={Box::new(move |ev| {
						email_input.set(event_target_value(&ev))
					})}
				/>

				<Show when={move || !email_error.get().is_empty()}>
					<Alert r#type={AlertType::Error} class="mt-xs">
						{move || email_error.get()}
					</Alert>
				</Show>

				<div class="w-full mt-xxs">
					{move || {
						show_coupon_button
							.get()
							.then(|| {
								view! {
									<Link
										on_click={Rc::new(move |_| {
											show_coupon.update(|val| *val = !*val)
										})}

										class="ml-auto"
									>

										{if show_coupon.get() {
											"Cancel"
										} else {
											"Have a Coupon Code?"
										}}

									</Link>
								}
									.into_view()
							})
					}}
					{move || {
						show_coupon
							.get()
							.then(|| {
								view! {
									<Input
										id="class"
										placeholder="Coupon Code"
										class="w-full mt-xs"
										start_icon={Some(
											IconProps::builder().icon(IconType::Tag).build(),
										)}
									/>
								}
							})
					}}

				</div>

				<Input
					r#type={InputType::Password}
					id="password"
					name="password"
					placeholder="Password"
					class="w-full mt-xxs"
					start_icon={Some(
						IconProps::builder().icon(IconType::Unlock).size(Size::Small).build(),
					)}
					value={password_input}
					on_input={Box::new(move |ev| {
						password_input.set(event_target_value(&ev));
					})}
				/>
				<Show when={move || !password_error.get().is_empty()}>
					<Alert r#type={AlertType::Error} class="mt-xs">
						{move || password_error.get()}
					</Alert>
				</Show>

				<Input
					r#type={InputType::Password}
					id="confirmPassword"
					placeholder="Confirm Password"
					class="w-full mt-lg"
					value={password_confirm_input}
					start_icon={Some(
						IconProps::builder().icon(IconType::Lock).size(Size::Small).build(),
					)}

					on_input={Box::new(move |ev| {
						password_confirm_input.set(event_target_value(&ev));
					})}
				/>

				<Show when={move || passwords_match.get()}>
					<Alert r#type={AlertType::Error} class="mt-xs">
						"Passwords Don't Match"
					</Alert>
				</Show>

				<div class="fr-fe-ct w-full mt-lg">
					<Link class="btn mr-xs" to="/confirm" r#type={Variant::Link}>
						"ALREADY HAVE AN OTP"
					</Link>

					<Show
						when=move || !loading.get()
						fallback=move || view! {
							<Link
								disabled={true}
								r#type={Variant::Button}
								style_variant={LinkStyleVariant::Contained}
							>
								"LOADING..."
							</Link>
						}
					>
						<Link
							disabled={Signal::derive(move || passwords_match.get() || loading.get())}
							r#type={Variant::Button}
							should_submit=true
							style_variant={LinkStyleVariant::Contained}
						>
							"NEXT"
						</Link>
					</Show>
				</div>
			</form>
		</div>
	}
}
