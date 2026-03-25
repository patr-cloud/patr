import { createFileRoute, Link, useNavigate } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import { createSignal, onMount, Show } from "solid-js";
import { CompleteSignUpRequest } from "~/bindings";
import { Button, ButtonVariant, useToast, Turnstile } from "~/components";
import Input, { InputType } from "~/components/input";
import OtpInput from "~/components/otp-input";
import { createAsyncAction } from "~/hooks";
import { httpRequest } from "~/utils/http-request";

const ConfirmSignUp = () => {
	const navigate = useNavigate();
	const toast = useToast();

	const search = Route.useSearch();
	const [turnstileToken, setTurnstileToken] = createSignal<string>("");

	// Get username from URL params or navigation state
	const initialUsername = search().username || "";
	const initialOtp = search().otp || "";

	// Track if username was pre-filled (from URL or navigation state)
	const usernameWasPreFilled = !!initialUsername;

	const [username, setUsername] = createSignal(initialUsername);
	const [otpDigits, setOtpDigits] = createSignal<string[]>(
		initialOtp
			? [...initialOtp.replace(/\D/g, "").slice(0, 6).padEnd(6, " ")].map((c) => (c === " " ? "" : c))
			: ["", "", "", "", "", ""]
	);

	// Clear URL params after reading them
	onMount(() => {
		if (search().username || search().otp) {
			navigate({ to: "/confirm-signup", search: { username: undefined, otp: undefined }, replace: true });
		}
	});

	const { execute: submitConfirmation, isLoading } = createAsyncAction(async () => {
		if (!turnstileToken()) {
			toast("Please complete the security verification", "error");
			return;
		}

		const body: CompleteSignUpRequest = {
			username: username(),
			verificationToken: otpDigits().join(""),
			cfTurnstileToken: turnstileToken(),
		};
		const resp = await httpRequest("/api/auth/join", {
			method: "POST",
			body: JSON.stringify(body),
		});

		if (resp.ok) {
			console.log("Account confirmed successfully");
			navigate({ to: "/login" });
		} else {
			console.error("Error confirming account:", resp.data.error);
			toast("Error confirming account", "error");
		}
	});

	const onSubmit = async (e: Event) => {
		e.preventDefault();
		await submitConfirmation().catch(() => {});
	};

	return (
		<>
			<Title>Confirm Sign Up | Patr</Title>
			<form
				onSubmit={onSubmit}
				class="bg-secondary p-12 rounded-sm shadow-2xl w-full max-w-128 relative z-10 border border-secondary-medium"
			>
				{/* Header */}
				<div class="mb-10 items-center justify-between flex flex-row">
					<h1 class="font-bold text-2xl text-white">Confirm Sign Up</h1>
					<div class="flex flex-row items-end">
						<div class="text-gray-400 font-extralight text-sm mr-2">New User?</div>
						<Link class="text-primary font-thin text-sm hover:underline" to="/sign-up">
							Sign Up
						</Link>
					</div>
				</div>

				{/* Username Input - only show if username was not pre-filled */}
				<Show when={!usernameWasPreFilled}>
					<Input
						type={InputType.Text}
						placeholder="Username"
						value={username}
						onInput={(e) => setUsername(e.currentTarget.value)}
						styleVariant="medium"
					/>
				</Show>

				{/* Show username as text if it was pre-filled */}
				<Show when={usernameWasPreFilled}>
					<div class="text-gray-400 text-sm">
						Confirming account for <span class="text-white font-medium">{username()}</span>
					</div>
				</Show>

				{/* OTP Input - 6 digits */}
				<div class="mt-8">
					<p class="text-gray-400 text-xs mb-2">Enter the 6-digit code sent to you</p>
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

				{/* Sign Up Button */}
				<div class="pt-8 w-full flex flex-row items-center justify-between">
					<Link to="/sign-up" class="text-primary text-xs hover:underline font-light">
						Back to Sign Up
					</Link>
					<Button
						loading={isLoading}
						loadingContent={() => <span>Confirming...</span>}
						variant={ButtonVariant.Contained}
						class="py-4 text-base font-semibold px-xxl flex-end transition-all duration-200"
						type="submit"
						disabled={!turnstileToken() || otpDigits().some((d) => d === "")}
					>
						Confirm
					</Button>
				</div>
			</form>
		</>
	);
};

export const Route = createFileRoute("/_logged-out/confirm-signup")({
	validateSearch: (search: Record<string, unknown>): { username?: string; otp?: string } => ({
		username: (search.username as string) || undefined,
		otp: (search.otp as string) || undefined,
	}),
	component: ConfirmSignUp,
});
