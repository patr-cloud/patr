import { createFileRoute, Link } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import { createSignal, Show } from "solid-js";
import { Alert, Button, Input, InputType, useToast, Turnstile } from "~/components";
import { ButtonVariant } from "~/utils/color";
import { httpRequest } from "~/utils/http-request";
import { createAsyncAction } from "~/hooks";

const ForgotPassword = () => {
	const toast = useToast();
	const [userId, setUserId] = createSignal("");
	const [userIdError, setUserIdError] = createSignal("");
	const [submitted, setSubmitted] = createSignal(false);
	const [turnstileToken, setTurnstileToken] = createSignal<string>("");

	const { execute: handleSubmit, isLoading } = createAsyncAction(async () => {
		if (!userId().trim()) {
			setUserIdError("Username or email is required.");
			return;
		}

		if (!turnstileToken()) {
			toast("Please complete the security verification", "error");
			return;
		}

		const resp = await httpRequest("/api/auth/forgot-password", {
			method: "POST",
			body: JSON.stringify({
				userId: userId(),
				preferredRecoveryOption: "recoveryEmail",
				cfTurnstileToken: turnstileToken(),
			}),
		});

		if (resp.ok) {
			setSubmitted(true);
		} else {
			toast("Failed to send reset link. Please try again.", "error");
		}
	});

	return (
		<>
			<Title>Reset Password | Patr</Title>
			<form
				noValidate
				class="bg-secondary p-12 rounded-sm shadow-2xl w-full max-w-128 relative z-10 border border-secondary-medium"
				onSubmit={async (e) => {
					e.preventDefault();
					await handleSubmit().catch(() => {
						toast("An unexpected error occurred. Please try again.", "error");
					});
				}}
			>
				{/* Header */}
				<div class="mb-10 items-center justify-between flex flex-row">
					<h1 class="font-bold text-2xl text-white">Reset Password</h1>
					<div class="flex flex-row items-baseline">
						<div class="text-gray-400 font-extralight text-sm mr-2">Remember it?</div>
						<Link class="text-primary font-thin text-sm hover:underline" to="/login">
							Login
						</Link>
					</div>
				</div>

				<Show
					when={!submitted()}
					fallback={
						/* Success Message */
						<div class="text-center">
							<div class="mb-6">
								<div class="w-16 h-16 bg-success/20 rounded-full flex items-center justify-center mx-auto mb-4">
									<svg
										class="w-8 h-8 text-success"
										fill="none"
										stroke="currentColor"
										viewBox="0 0 24 24"
									>
										<path
											stroke-linecap="round"
											stroke-linejoin="round"
											stroke-width="2"
											d="M3 8l7.89 5.26a2 2 0 002.22 0L21 8M5 19h14a2 2 0 002-2V7a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z"
										/>
									</svg>
								</div>
								<h2 class="text-xl font-semibold text-white mb-2">Check Your Email</h2>
								<p class="text-grey text-sm">
									We've sent password reset instructions to the recovery email for&nbsp;
									<span class="text-white font-medium">{userId()}</span>
								</p>
							</div>
							<div class="mt-8 pt-6 border-t border-border-color">
								<p class="text-grey text-sm mb-4">
									Didn't receive the email? Check your spam folder or
								</p>
								<button
									type="button"
									onClick={() => setSubmitted(false)}
									class="text-primary font-medium hover:underline text-sm"
								>
									Try again
								</button>
							</div>
						</div>
					}
				>
					{/* Form */}
					<div>
						<Input
							type={InputType.Text}
							placeholder="Username or email"
							autocomplete="username"
							required={true}
							name="userId"
							id="userId"
							value={userId}
							onInput={(e: Event) => {
								setUserId((e.currentTarget as HTMLInputElement).value);
								setUserIdError("");
							}}
							styleVariant="medium"
						/>
						<Show when={userIdError()}>
							<div class="mt-1">
								<Alert message={userIdError()} type="error" />
							</div>
						</Show>

						{/* Turnstile Widget */}
						<div class="mt-6 flex justify-center">
							<Turnstile
								onVerify={setTurnstileToken}
								onExpire={() => setTurnstileToken("")}
								onError={() => setTurnstileToken("")}
								action="forgot-password"
							/>
						</div>

						{/* Submit Button */}
						<div class="pt-8 w-full flex flex-row items-center justify-end">
							<Button
								variant={ButtonVariant.Contained}
								class="py-4 text-base font-semibold px-8"
								type="submit"
								loading={isLoading}
								loadingContent={() => <span>Sending...</span>}
								disabled={!turnstileToken()}
							>
								Send Reset Link
							</Button>
						</div>
					</div>
				</Show>
			</form>

			{/* Footer */}
			<div class="absolute bottom-6 left-0 right-0 text-center">
				<p class="text-gray-500 text-xs">&copy; {new Date().getFullYear()} Patr. All rights reserved.</p>
			</div>
		</>
	);
};

export const Route = createFileRoute("/_logged-out/forgot-password")({
	component: ForgotPassword,
});
