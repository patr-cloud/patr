import { createFileRoute, Link, useNavigate } from "@tanstack/solid-router";
import { createSignal } from "solid-js";
import { CreateAccountRequest } from "~/bindings";
import { Button, useToast, Turnstile } from "~/components";
import Input, { InputType } from "~/components/input";
import { createAsyncAction } from "~/hooks";
import { ButtonVariant } from "~/utils/color";
import { httpRequest } from "~/utils/http-request";

const SignUp = () => {
	const toast = useToast();
	const navigate = useNavigate();

	const [username, setUsername] = createSignal("");
	const [firstName, setFirstName] = createSignal("");
	const [lastName, setLastName] = createSignal("");

	const [email, setEmail] = createSignal("");
	const [password, setPassword] = createSignal("");
	const [confirmPassword, setConfirmPassword] = createSignal("");

	const [turnstileToken, setTurnstileToken] = createSignal<string>("");

	const { execute: submitSignUp, isLoading } = createAsyncAction(async () => {
		if (!turnstileToken()) {
			toast("Please complete the security verification", "error");
			return;
		}

		const requestBody: CreateAccountRequest = {
			username: username(),
			password: password(),
			firstName: firstName(),
			lastName: lastName(),
			recoveryEmail: email(),
			cfTurnstileToken: turnstileToken(),
		};

		const resp = await httpRequest("/api/auth/sign-up", {
			method: "POST",
			body: JSON.stringify(requestBody),
		});

		if (resp.ok) {
			// Handle successful sign-up (e.g., redirect to login or dashboard)
			console.log("Account created successfully");
			navigate({ to: "/confirm-signup", search: { username: username(), otp: undefined } });
		} else {
			// Handle sign-up errors
			console.error("Error creating account:", resp.statusText);
			toast("Error creating account: " + resp.statusText, "error");
		}
	});

	return (
		<>
			{/* Sign Up Card */}
			<form
				onSubmit={async (e) => {
					e.preventDefault();
					await submitSignUp().catch((e) => {
						toast("An unexpected error occurred. Please try again.", "error");
						console.error("Unexpected error during sign-up:", e);
						setTurnstileToken("");
					});
				}}
				class="bg-secondary p-12 rounded-sm shadow-2xl w-full max-w-128 relative z-10 border border-secondary-medium"
			>
				{/* Header */}
				<div class="mb-10 items-center justify-between flex flex-row">
					<h1 class="font-bold text-2xl text-white">Sign Up</h1>
					<div class="flex flex-row items-end">
						<div class="text-gray-400 font-extralight text-sm mr-2">Already a User?</div>
						<Link class="text-primary font-thin text-sm hover:underline" to="/login">
							Login
						</Link>
					</div>
				</div>

				{/* Form */}
				<div>
					<Input
						type={InputType.Text}
						placeholder="Username"
						name="username"
						id="username"
						value={username}
						onInput={(e) => setUsername(e.currentTarget.value)}
						styleVariant="medium"
					/>

					{/* Name Inputs */}
					<div class="flex items-center gap-4 mt-4">
						<Input
							type={InputType.Text}
							placeholder="First Name"
							required={true}
							name="first-name"
							id="first-name"
							value={firstName}
							onInput={(e) => setFirstName(e.currentTarget.value)}
							styleVariant="medium"
							class="flex-1"
						/>
						<Input
							type={InputType.Text}
							placeholder="Last Name"
							required={true}
							name="last-name"
							id="last-name"
							value={lastName}
							onInput={(e) => setLastName(e.currentTarget.value)}
							styleVariant="medium"
							class="flex-1"
						/>
					</div>

					<Input
						type={InputType.Email}
						placeholder="Email Address"
						value={email}
						onInput={(e) => setEmail(e.currentTarget.value)}
						class="mt-4"
						styleVariant="medium"
					/>

					<Input
						type={InputType.Password}
						placeholder="Password"
						value={password}
						onInput={(e) => setPassword(e.currentTarget.value)}
						class="mt-4"
						styleVariant="medium"
					/>

					<Input
						type={InputType.Password}
						placeholder="Confirm Password"
						value={confirmPassword}
						onInput={(e) => setConfirmPassword(e.currentTarget.value)}
						class="mt-4"
						styleVariant="medium"
					/>

					{/* Turnstile Widget */}
					<div class="mt-6 flex justify-center">
						<Turnstile
							onVerify={setTurnstileToken}
							onExpire={() => setTurnstileToken("")}
							onError={() => setTurnstileToken("")}
							action="sign-up"
						/>
					</div>

					{/* Sign Up Button */}
					<div class="pt-8 w-full flex flex-row items-center justify-between">
						<Link to="/confirm-signup" search={{ username: undefined, otp: undefined }} class="text-primary text-xs hover:underline font-light">
							Have an OTP?
						</Link>
						<Button
							variant={ButtonVariant.Contained}
							class="py-4 text-base font-semibold px-xxl flex-end"
							type="submit"
							loading={isLoading}
							loadingContent={() => <span>Signing up...</span>}
						>
							Sign Up
						</Button>
					</div>
				</div>
			</form>

			{/* Footer */}
			<div class="absolute bottom-6 left-0 right-0 text-center">
				<p class="text-gray-500 text-xs">&copy; 2025 Patr. All rights reserved.</p>
			</div>
		</>
	);
};

export const Route = createFileRoute("/_auth/sign-up")({
	component: SignUp,
});
