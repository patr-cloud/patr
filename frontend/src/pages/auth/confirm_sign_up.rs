use crate::prelude::*;

#[component]
pub fn ConfirmSignUp(
	/// The scope of the component.
	_cx: Scope,
) -> impl IntoView {
	// let navigate = use_navigate(cx);

	// let confirm_sign_up_action = create_action(cx, |(username, otp): &(String, String)| {
	// 	let username = username.clone();
	// 	let otp = otp.clone();
	// 	async move {
	// 		let result = make_request(
	// 			ApiRequest::<CompleteSignUpRequest>::builder()
	// 				.path(Default::default())
	// 				.query(())
	// 				.headers(())
	// 				.body(CompleteSignUpRequest {
	// 					username,
	// 					verification_token: otp,
	// 				})
	// 				.build(),
	// 		)
	// 		.await;
	// 	}
	// });

	// let confirm_loading = confirm_sign_up_action.pending();

	// view! { cx,
	// 	<div class="box-onboard fc-fs-fs">
	// 		<div class="fr-sb-bl mb-lg full-width">
	// 			<h1 class="txt-primary txt-xl txt-medium">Confirm OTP</h1>
	// 			<Link
	// 				disabled=confirm_loading
	// 				on_click=Box::new(move |_| {
	// 					_ = navigate(
	// 						AppRoute::LoggedOutRoutes(LoggedOutRoutes::SignUp).to_string().as_str(),
	// 						NavigateOptions::default(),
	// 					);
	// 				})
	// 			>
	// 				Sign Up with different Email
	// 			</Link>
	// 		</div>
	// 		<form on:submit={handle_confirm_sign_up} class="fc-fs-fs full-width">
	// 			{move || {
	// 				username
	// 					.get()
	// 					.is_empty()
	// 					.then(|| view!{ cx,
	// 						<Input
	// 							value={username.get()}
	// 							disabled={confirm_loading}
	// 							placeholder="Username"
	// 							id="username"
	// 							class="full-width"
	// 							/>
	// 					})
	// 			}}
	// 			{move || {
	// 				username_error
	// 					.get()
	// 					.some_if_not_empty()
	// 					.map(|error| view! { cx,
	// 						<Alert
	// 							message=error
	// 							r#type=NotificationType::Error
	// 							class="mt-xs"
	// 							/>
	// 					})
	// 			}}
	// 			<span class="mb-xxs mt-sm txt-sm txt-white">Enter OTP</span>
	// 			<OtpInput
	// 				otp=otp
	// 				on_submit=Rc::new(move |_| {
	// 					handle_confirm_sign_up(
	// 						ev::SubmitEvent::new("submit").unwrap()
	// 					);
	// 				})
	// 				disabled={confirm_loading}
	// 				/>
	// 			{move || {
	// 				otp_error
	// 					.get()
	// 					.some_if_not_empty()
	// 					.map(|error| view! { cx,
	// 						<Alert
	// 							message=error
	// 							r#type=NotificationType::Error
	// 							class="mt-xs"
	// 							/>
	// 					})
	// 			}}
	// 			<div class="fr-fe-ct full-width mt-lg">
	// 				{resendLoading ? (
	// 					<Spinner class="spinner-xs mx-xl" />
	// 				) : (
	// 					defaultPassword && (
	// 						<Link
	// 							disabled={confirm_loading}
	// 							on_click={handleResendOtp}
	// 							class="btn mr-xs"
	// 						>
	// 							RESEND OTP
	// 						</Link>
	// 					)
	// 				)}
	// 				{move || if confirm_loading.get() || resend_loading.get() {
	// 					view! {cx,
	// 						<Spinner class="mx-xl" />
	// 					}
	// 				} else {
	// 					view! {cx,
	// 						<Link
	// 							r#type="submit"
	// 							variant=LinkVariant::Contained>
	// 							SIGN UP
	// 						</Link>
	// 					}
	// 				}}
	// 			</div>
	// 		</form>
	// 	</div>
	// }
}
