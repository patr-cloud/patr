use std::rc::Rc;

use leptos_use::{use_cookie, utils::FromToStringCodec};
use models::api::user::{ActivateMfaResponse, ChangePasswordResponse};

use crate::prelude::*;

#[server(ChangePasswordFn, endpoint = "/user/change-password")]
async fn change_password(
	access_token: Option<String>,
	mfa_otp: Option<String>,
	new_password: String,
	current_password: String,
) -> Result<Result<ChangePasswordResponse, ErrorType>, ServerFnError> {
	use std::str::FromStr;

	use models::api::user::{
		ChangePasswordPath,
		ChangePasswordRequest,
		ChangePasswordRequestHeaders,
	};

	let api_response = make_api_call::<ChangePasswordRequest>(
		ApiRequest::builder()
			.path(ChangePasswordPath)
			.query(())
			.headers(ChangePasswordRequestHeaders {
				authorization: BearerToken::from_str(
					format!("Bearer {}", access_token.unwrap_or_default()).as_str(),
				)?,
				user_agent: UserAgent::from_static("hyper/0.12.2"),
			})
			.body(ChangePasswordRequest {
				current_password,
				new_password,
				mfa_otp,
			})
			.build(),
	)
	.await;

	Ok(api_response.map(|res| res.body))
}

///
#[server(ActivateMfaFn, endpoint = "/user/mfa")]
async fn activate_mfa(
	access_token: Option<String>,
	otp: String,
) -> Result<Result<ActivateMfaResponse, ErrorType>, ServerFnError> {
	use std::str::FromStr;

	use models::api::user::{ActivateMfaPath, ActivateMfaRequest, ActivateMfaRequestHeaders};

	let api_response = make_api_call::<ActivateMfaRequest>(
		ApiRequest::builder()
			.path(ActivateMfaPath)
			.query(())
			.headers(ActivateMfaRequestHeaders {
				authorization: BearerToken::from_str(
					format!("Bearer {}", access_token.unwrap_or_default()).as_str(),
				)?,
			})
			.body(ActivateMfaRequest { otp })
			.build(),
	)
	.await;

	Ok(api_response.map(|res| res.body))
}

#[component]
pub fn PasswordSection() -> impl IntoView {
	let show_create_password_fields = create_rw_signal(false);

	let change_password_action = create_server_action::<ChangePasswordFn>();
	let response = change_password_action.value();

	let (access_token, _) = use_cookie::<String, FromToStringCodec>("access_token");

	let current_password_error = create_rw_signal("".to_owned());
	let _new_password_error = create_rw_signal("".to_owned());
	let confirm_password_error = create_rw_signal("".to_owned());

	let handle_errors = move |error: ErrorType| match error {
		ErrorType::InvalidPassword => {
			current_password_error.set("Password is Incorrect".to_owned());
		}
		e => {
			confirm_password_error.set(format!("{:?}", e));
		}
	};

	let open_mfa_modal = create_rw_signal(false);

	create_effect(move |_| {
		if let Some(Ok(resp)) = response.get() {
			let _ = match resp {
				Ok(ChangePasswordResponse {}) => {}
				Err(err) => {
					logging::log!("{:#?}", err);
					handle_errors(err);
					return;
				}
			};
		}
	});

	view! {
		<div class="txt-white fc-fs-fs full-width px-xl py-lg br-sm bg-secondary-light">
			<div class="fr-fs-ct full-width pb-sm ul-light">
				<h2 class="letter-sp-md txt-md">Security</h2>
			</div>

			<form class="full-width pt-md gap-md fc-fs-fs">
				<Show when={move || open_mfa_modal.get()}>
					<Modal variant={SecondaryColorVariant::Medium}>
						<h1>"Scan the QR Code to enable 2FA"</h1>

						<form class="full-width flex mb-md">
							<p class="flex-col-7 br-light pr-xl py-xl">
								"Enhance the security of your account by enabling Two-Factor
								Authentication (2FA). Scan the generated QR code using an
								authenticator app on your mobile device to set up the additional
								layer of protection."
							</p>

							// QR CODE
							<div class="flex-col-5 fr-ct-ct"></div>

							<small class="txt-warning">
								"Note: Enabling 2FA will temporarily disable container registry
								access via username and password. Instead, generate and use an API
								token for accessing the container registry after enabling 2FA."
							</small>

							<p class="mt-md mb-xs txt-sm">"Enter OTP to confirm"</p>

							<Input r#type={InputType::Text} id="otp"/>

							<div>
								<Link
									on_click={Rc::new(move |_| { open_mfa_modal.set(false) })}

									style_variant={LinkStyleVariant::Plain}
									should_submit=false
								>
									"CANCEL"
								</Link>
								<Link
									style_variant={LinkStyleVariant::Contained}
									should_submit=true
								>
									"CONFIRM"
								</Link>
							</div>
						</form>
					</Modal>
				</Show>

				<div class="flex full-width px-md">
					<div class="flex-col-2 fr-fs-fs pt-sm txt-sm">
						"Two-Factor" <br/> "Authentication"
					</div>

					<div class="flex-col-10 fc-fs-fs gap-xxs">
						<p class="w-70">
							"Two-Factor Authentication (2FA) adds an extra layer of security to your account by requiring a second form of verification in addition to your password. By enabling 2FA, you'll be prompted to authenticate your login using a unique code generated by an authenticator app, ensuring that only you can access your account."
						</p>

						<Link
							on_click={Rc::new(move |_| { open_mfa_modal.set(true) })}

							style_variant={LinkStyleVariant::Contained}
						>
							"ENABLE 2FA"
						</Link>
					</div>
				</div>
			</form>

			<ActionForm action={change_password_action} class="full-width pt-md gap-md fc-fs-fs">
				<input type="hidden" name="mfa_otp"/>
				<input type="hidden" name="access_token" prop:value={access_token}/>

				<Show
					when={move || show_create_password_fields.get()}
					fallback={|| {
						view! {
							<div class="flex full-width px-md">
								<div class="flex-col-2 fr-fs-fs">
									<label html_for="password" class="mt-sm txt-sm">
										"Password"
									</label>
								</div>

								<div class="flex-col-10 fc-fs-fs">
									<Input
										id="password"
										placeholder="********"
										disabled=true
										class="full-width"
										end_icon={None}
										r#type={InputType::Password}
										variant={SecondaryColorVariant::Medium}
									/>
								</div>
							</div>
						}
					}}
				>

					<div class="flex full-width px-md">
						<div class="flex-col-2 fr-fs-fs">
							<label html_for="currentPassword" class="mt-sm txt-sm">
								"Current Password"
							</label>
						</div>

						<div class="flex-col-10 fc-fs-fs">
							<Input
								id="currentPassword"
								name="current_password"
								placeholder="Enter Current Password"
								class="full-width"
								end_icon={None}
								r#type={InputType::Password}
								variant={SecondaryColorVariant::Medium}
							/>

							<Show when={move || !current_password_error.get().is_empty()}>
								<Alert r#type={AlertType::Error} class="mt-xs">
									{move || current_password_error.get()}
								</Alert>
							</Show>
						</div>
					</div>

					<div class="flex full-width px-md">
						<div class="flex-col-2 fr-fs-fs">
							<label html_for="newPassword" class="mt-sm txt-sm">
								"New Password"
							</label>
						</div>

						<div class="flex-col-10 fc-fs-fs">
							<Input
								id="newPassword"
								placeholder="Enter New Password"
								class="full-width"
								end_icon={None}
								r#type={InputType::Password}
								variant={SecondaryColorVariant::Medium}
							/>
						</div>
					</div>

					<div class="flex full-width px-md">
						<div class="flex-col-2 fr-fs-fs">
							<label html_for="confirmPassword" class="mt-sm txt-sm">
								"Confirm New Password"
							</label>
						</div>

						<div class="flex-col-10 fc-fs-fs">
							<Input
								id="confirmNewPassword"
								name="new_password"
								placeholder="Confirm New Password"
								class="full-width"
								end_icon={None}
								r#type={InputType::Password}
								variant={SecondaryColorVariant::Medium}
							/>
						</div>
					</div>
				</Show>

				<Show
					when={move || show_create_password_fields.get()}
					fallback={move || {
						view! {
							<div class="full-width fr-fe-ct pt-md">
								<Link
									on_click={Rc::new(move |_| {
										show_create_password_fields.update(|val| *val = !*val)
									})}

									should_submit=false
									style_variant={LinkStyleVariant::Contained}
								>
									"CHANGE PASSWORD"
								</Link>
							</div>
						}
					}}
				>

					<div class="full-width fr-fe-ct pt-md gap-md">
						<Link
							on_click={Rc::new(move |_| {
								show_create_password_fields.update(|val| *val = !*val)
							})}

							should_submit=false
							style_variant={LinkStyleVariant::Plain}
						>
							"CANCEL"
						</Link>

						<Link should_submit=true style_variant={LinkStyleVariant::Contained}>
							"CONFIRM"
						</Link>
					</div>
				</Show>
			</ActionForm>
		</div>
	}
}
