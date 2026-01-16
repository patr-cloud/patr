import { A } from "@solidjs/router";
import { createSignal } from "solid-js";
import Button from "~/components/button";
import Input, { InputType } from "~/components/input";
import { ButtonVariant } from "~/utils/color";

const ForgotPassword = () => {
	const [email, setEmail] = createSignal("");
	const [submitted, setSubmitted] = createSignal(false);

	const handleSubmit = () => {
		setSubmitted(true);
	};

	return (
		<main class="min-h-screen w-full bg-secondary flex items-center justify-center p-4 relative overflow-hidden">
			{/* Background decorative elements */}
			<div class="absolute inset-0 overflow-hidden pointer-events-none">
				<div class="absolute inset-0 w-full h-full bg-linear-to-br from-secondary via-secondary-dark to-secondary opacity-50"></div>
			</div>

			{/* Forgot Password Card */}
			<section class="bg-secondary-dark p-12 rounded-2xl shadow-2xl w-full max-w-[480px] relative z-10 border border-secondary-medium">
				{/* Logo */}
				<div class="flex justify-center mb-10">
					<div class="text-primary text-4xl font-bold">PATR</div>
				</div>

				{submitted() ? (
					/* Success Message */
					<div class="text-center">
						<div class="mb-6">
							<div class="w-16 h-16 bg-success bg-opacity-20 rounded-full flex items-center justify-center mx-auto mb-4">
								<svg class="w-8 h-8 text-success" fill="none" stroke="currentColor" viewBox="0 0 24 24">
									<path
										stroke-linecap="round"
										stroke-linejoin="round"
										stroke-width="2"
										d="M3 8l7.89 5.26a2 2 0 002.22 0L21 8M5 19h14a2 2 0 002-2V7a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z"
									/>
								</svg>
							</div>
							<h1 class="text-2xl font-semibold text-white mb-2">Check Your Email</h1>
							<p class="text-gray-400 text-sm">
								We've sent password reset instructions to
								<span class="text-primary font-medium">{email()}</span>
							</p>
						</div>

						<div class="mt-8 pt-6 border-t border-gray-600">
							<p class="text-gray-400 text-sm mb-4">Didn't receive the email? Check your spam folder or</p>
							<button onClick={() => setSubmitted(false)} class="text-primary font-medium hover:underline text-sm">
								Try again
							</button>
						</div>

						<div class="mt-6">
							<A href="/login" class="text-gray-400 text-sm hover:text-primary flex items-center justify-center gap-2">
								<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
									<path
										stroke-linecap="round"
										stroke-linejoin="round"
										stroke-width="2"
										d="M10 19l-7-7m0 0l7-7m-7 7h18"
									/>
								</svg>
								Back to Sign In
							</A>
						</div>
					</div>
				) : (
					/* Reset Form */
					<>
						<div class="text-center mb-10">
							<h1 class="text-4xl font-bold text-white mb-3">Reset Password</h1>
							<p class="text-gray-400 text-base">Enter your email to receive reset instructions</p>
						</div>

						<div class="space-y-6">
							<div class="space-y-2">
								<label class="text-white text-sm font-medium block pl-1">Email Address</label>
								<Input
									type={InputType.Email}
									placeholder="Enter your email"
									value={email}
									onInput={(e: Event) => setEmail((e.currentTarget as HTMLInputElement).value)}
									styleVariant="medium"
								/>
							</div>

							<div class="pt-4">
								<Button
									variant={ButtonVariant.Contained}
									class="w-full py-4 text-base font-semibold"
									onClick={handleSubmit}
								>
									Send Reset Link
								</Button>
							</div>
						</div>

						<div class="mt-8 text-center">
							<A href="/login" class="text-gray-400 text-sm hover:text-primary flex items-center justify-center gap-2">
								<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
									<path
										stroke-linecap="round"
										stroke-linejoin="round"
										stroke-width="2"
										d="M10 19l-7-7m0 0l7-7m-7 7h18"
									/>
								</svg>
								Back to Sign In
							</A>
						</div>
					</>
				)}
			</section>

			{/* Footer */}
			<div class="absolute bottom-6 left-0 right-0 text-center">
				<p class="text-gray-500 text-xs">© 2025 Patr. All rights reserved.</p>
			</div>
		</main>
	);
};

export default ForgotPassword;
