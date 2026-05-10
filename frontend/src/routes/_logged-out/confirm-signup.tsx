import { createFileRoute, Link, useNavigate } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import { createSignal, onMount, Show } from "solid-js";
import { CompleteSignUpRequest } from "~/bindings";
import { Alert, Button, ButtonVariant, Input, InputType, OtpInput, useToast, Turnstile } from "~/components";
import { createAsyncAction } from "~/hooks";
import { httpRequest } from "~/utils/http-request";

const ConfirmSignUp = () => {
	const navigate = useNavigate();
	const toast = useToast();

	const search = Route.useSearch();
	const [turnstileToken, setTurnstileToken] = createSignal<string>("");

	const initialEmail = search().email || "";
	const initialOtp = search().otp || "";

	const emailWasPreFilled = !!initialEmail;

	const [email, setEmail] = createSignal(initialEmail);
	const [emailError, setEmailError] = createSignal("");
	const [otpDigits, setOtpDigits] = createSignal<string[]>(
		initialOtp
			? [...initialOtp.replace(/\D/g, "").slice(0, 6).padEnd(6, " ")].map((c) => (c === " " ? "" : c))
			: ["", "", "", "", "", ""]
	);

	onMount(() => {
		if (search().email || search().otp) {
			navigate({ to: "/confirm-signup", search: { email: undefined, otp: undefined }, replace: true });
		}
	});

	const { execute: submitConfirmation, isLoading } = createAsyncAction(async () => {
		if (!email().trim()) {
			setEmailError("Email is required.");
			return;
		}

		if (!turnstileToken()) {
			toast("Please complete the security verification", "error");
			return;
		}

		const body: CompleteSignUpRequest = {
			email: email(),
			verificationToken: otpDigits().join(""),
			cfTurnstileToken: turnstileToken(),
		};
		const resp = await httpRequest("/api/auth/join", {
			method: "POST",
			body: JSON.stringify(body),
		});

		if (resp.ok) {
			toast("Account verified! You can now log in.", "success");
			navigate({ to: "/login" });
		} else {
			const errorMsg =
				resp.data.error === "userNotFound"
					? "Those credentials don't match our records. Please check your email and verification code."
					: "Error confirming account. Please try again.";
			toast(errorMsg, "error");
		}
	});

	const handleResendOtp = () => {
		toast("To get a new code, please sign up again with the same email.", "info");
		navigate({ to: "/sign-up" });
	};

	const onSubmit = async (e: Event) => {
		e.preventDefault();
		await submitConfirmation().catch(() => {
			toast("An unexpected error occurred. Please try again.", "error");
		});
	};

	return (
		<>
			<Title>Confirm Sign Up | Patr</Title>
			<form
				noValidate
				onSubmit={onSubmit}
				class="bg-secondary p-12 rounded-sm shadow-2xl w-full max-w-128 relative z-10 border border-secondary-medium"
			>
				{/* Header */}
				<div class="mb-10 items-center justify-between flex flex-row">
					<h1 class="font-bold text-2xl text-white">Confirm Sign Up</h1>
					<div class="flex flex-row items-baseline">
						<div class="text-gray-400 font-extralight text-sm mr-2">Already verified?</div>
						<Link class="text-primary font-thin text-sm hover:underline" to="/login">
							Login
						</Link>
					</div>
				</div>

				{/* Email Input - only show if not pre-filled */}
				<Show when={!emailWasPreFilled}>
					<Input
						type={InputType.Email}
						placeholder="Email"
						required={true}
						autocomplete="email"
						name="email"
						id="email"
						value={email}
						onInput={(e) => {
							setEmail(e.currentTarget.value);
							setEmailError("");
						}}
						styleVariant="medium"
					/>
					<Show when={emailError()}>
						<div class="mt-1">
							<Alert message={emailError()} type="error" />
						</div>
					</Show>
				</Show>

				<Show when={emailWasPreFilled}>
					<div class="text-gray-400 text-sm">
						Confirming account for <span class="text-white font-medium">{email()}</span>
					</div>
				</Show>

				{/* OTP Input - 6 digits */}
				<div class="mt-8">
					<p class="text-grey text-sm mb-3">Enter the 6-digit verification code sent to your email</p>
					<OtpInput inputVariant="medium" otpDigits={otpDigits} setOtpDigits={setOtpDigits} />
				</div>

				{/* Turnstile Widget */}
				<div class="mt-6 flex justify-center">
					<Turnstile
						onVerify={setTurnstileToken}
						onExpire={() => setTurnstileToken("")}
						onError={() => setTurnstileToken("")}
						action="complete-sign-up"
					/>
				</div>

				{/* Buttons */}
				<div class="pt-8 w-full flex flex-row items-center justify-between">
					<div class="flex items-center gap-4">
						<Link to="/sign-up" class="text-primary text-xs hover:underline font-light">
							Back to Sign Up
						</Link>
						<button
							type="button"
							class="text-grey text-xs hover:text-primary hover:underline font-light"
							onClick={handleResendOtp}
						>
							Resend Code
						</button>
					</div>
					<Button
						loading={isLoading}
						loadingContent={() => <span>Confirming...</span>}
						variant={ButtonVariant.Contained}
						class="py-4 text-base font-semibold px-8"
						type="submit"
						disabled={!turnstileToken() || otpDigits().some((d) => d === "")}
					>
						Confirm
					</Button>
				</div>
			</form>

			{/* Footer */}
			<div class="absolute bottom-6 left-0 right-0 text-center">
				<p class="text-gray-500 text-xs">&copy; {new Date().getFullYear()} Patr. All rights reserved.</p>
			</div>
		</>
	);
};

export const Route = createFileRoute("/_logged-out/confirm-signup")({
	validateSearch: (search: Record<string, unknown>): { email?: string; otp?: string } => ({
		email: (search.email as string) || undefined,
		otp: (search.otp as string) || undefined,
	}),
	component: ConfirmSignUp,
});
