import { createFileRoute, Link, useNavigate } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import { createSignal, Show } from "solid-js";
import { CreateAccountRequest, SocialLoginInitiateResponse } from "~/bindings";
import { Alert, Button, Input, InputType, useToast, Turnstile } from "~/components";
import { createAsyncAction } from "~/hooks";
import { ButtonVariant } from "~/utils/color";
import { httpRequest } from "~/utils/http-request";
import { validateNameField, validatePassword } from "~/utils/validation";

interface FieldErrors {
	username: string;
	firstName: string;
	lastName: string;
	email: string;
	password: string;
	confirmPassword: string;
}

const emptyErrors: FieldErrors = {
	username: "",
	firstName: "",
	lastName: "",
	email: "",
	password: "",
	confirmPassword: "",
};

const SignUp = () => {
	const toast = useToast();
	const navigate = useNavigate();
	const [githubLoading, setGithubLoading] = createSignal(false);

	const handleGithubSignIn = async () => {
		setGithubLoading(true);
		try {
			const resp = await httpRequest<SocialLoginInitiateResponse>("/api/auth/social-login/github", {
				method: "POST",
			});
			if (resp.ok) {
				window.location.href = resp.data.authorizeUrl;
				return;
			}

			toast("Could not initiate GitHub sign-in. Please try again.", "error");
		} catch {
			toast("Could not initiate GitHub sign-in. Please try again.", "error");
		} finally {
			setGithubLoading(false);
		}
	};

	const [username, setUsername] = createSignal("");
	const [firstName, setFirstName] = createSignal("");
	const [lastName, setLastName] = createSignal("");

	const [email, setEmail] = createSignal("");
	const [password, setPassword] = createSignal("");
	const [confirmPassword, setConfirmPassword] = createSignal("");

	const [turnstileToken, setTurnstileToken] = createSignal<string>("");
	const [errors, setErrors] = createSignal<FieldErrors>({ ...emptyErrors });

	const clearError = (field: keyof FieldErrors) => {
		setErrors((prev) => ({ ...prev, [field]: "" }));
	};

	const validateInputs = (): boolean => {
		const newErrors = { ...emptyErrors };
		let valid = true;

		if (!username().trim()) {
			newErrors.username = "Username is required.";
			valid = false;
		}
		const firstNameError = validateNameField(firstName());
		if (firstNameError) {
			newErrors.firstName = firstNameError;
			valid = false;
		}
		const lastNameError = validateNameField(lastName());
		if (lastNameError) {
			newErrors.lastName = lastNameError;
			valid = false;
		}
		if (!email().trim()) {
			newErrors.email = "Email is required.";
			valid = false;
		}
		if (!password()) {
			newErrors.password = "Password is required.";
			valid = false;
		} else {
			const passwordValidation = validatePassword(password());
			if (!passwordValidation.valid) {
				newErrors.password = passwordValidation.error || "Invalid password.";
				valid = false;
			}
		}
		if (password() !== confirmPassword()) {
			newErrors.confirmPassword = "Passwords do not match.";
			valid = false;
		}

		setErrors(newErrors);
		return valid;
	};

	const { execute: submitSignUp, isLoading } = createAsyncAction(async () => {
		if (!validateInputs()) return;

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
			navigate({ to: "/confirm-signup", search: { username: username(), otp: undefined } });
		} else {
			switch (resp.data.error) {
				case "usernameUnavailable":
					setErrors((prev) => ({ ...prev, username: "Username is already taken." }));
					break;
				case "emailUnavailable":
					setErrors((prev) => ({ ...prev, email: "Email is already in use." }));
					break;
				default:
					toast("Error creating account: " + resp.statusText, "error");
					break;
			}
		}
	});

	return (
		<>
			<Title>Sign Up | Patr</Title>
			{/* Sign Up Card */}
			<form
				noValidate
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
					<div class="flex flex-row items-baseline">
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
						autocomplete="username"
						required={true}
						name="username"
						id="username"
						value={username}
						onInput={(e) => {
							setUsername(e.currentTarget.value);
							clearError("username");
						}}
						styleVariant="medium"
					/>
					<Show when={errors().username}>
						<div class="mt-1">
							<Alert message={errors().username} type="error" />
						</div>
					</Show>

					{/* Name Inputs */}
					<div class="flex items-center gap-4 mt-4">
						<div class="flex-1">
							<Input
								type={InputType.Text}
								placeholder="First Name"
								autocomplete="given-name"
								required={true}
								name="first-name"
								id="first-name"
								value={firstName}
								onInput={(e) => {
									setFirstName(e.currentTarget.value);
									clearError("firstName");
								}}
								styleVariant="medium"
							/>
							<Show when={errors().firstName}>
								<div class="mt-1">
									<Alert message={errors().firstName} type="error" />
								</div>
							</Show>
						</div>
						<div class="flex-1">
							<Input
								type={InputType.Text}
								placeholder="Last Name"
								autocomplete="family-name"
								required={true}
								name="last-name"
								id="last-name"
								value={lastName}
								onInput={(e) => {
									setLastName(e.currentTarget.value);
									clearError("lastName");
								}}
								styleVariant="medium"
							/>
							<Show when={errors().lastName}>
								<div class="mt-1">
									<Alert message={errors().lastName} type="error" />
								</div>
							</Show>
						</div>
					</div>

					<Input
						type={InputType.Email}
						placeholder="Email Address"
						autocomplete="email"
						required={true}
						name="email"
						id="email"
						value={email}
						onInput={(e) => {
							setEmail(e.currentTarget.value);
							clearError("email");
						}}
						class="mt-4"
						styleVariant="medium"
					/>
					<Show when={errors().email}>
						<div class="mt-1">
							<Alert message={errors().email} type="error" />
						</div>
					</Show>

					<Input
						type={InputType.Password}
						placeholder="Password"
						autocomplete="new-password"
						required={true}
						name="password"
						id="password"
						value={password}
						onInput={(e) => {
							setPassword(e.currentTarget.value);
							clearError("password");
						}}
						class="mt-4"
						styleVariant="medium"
					/>
					<Show when={errors().password}>
						<div class="mt-1">
							<Alert message={errors().password} type="error" />
						</div>
					</Show>

					<Input
						type={InputType.Password}
						placeholder="Confirm Password"
						autocomplete="new-password"
						required={true}
						name="confirm-password"
						id="confirm-password"
						value={confirmPassword}
						onInput={(e) => {
							setConfirmPassword(e.currentTarget.value);
							clearError("confirmPassword");
						}}
						class="mt-4"
						styleVariant="medium"
					/>
					<Show when={errors().confirmPassword}>
						<div class="mt-1">
							<Alert message={errors().confirmPassword} type="error" />
						</div>
					</Show>

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
						<Link
							to="/confirm-signup"
							search={{ username: undefined, otp: undefined }}
							class="text-primary text-xs hover:underline font-light"
						>
							Have an OTP?
						</Link>
						<Button
							variant={ButtonVariant.Contained}
							class="py-4 text-base font-semibold px-8"
							type="submit"
							loading={isLoading}
							loadingContent={() => <span>Signing up...</span>}
							disabled={!turnstileToken()}
						>
							Sign Up
						</Button>
					</div>

					{/* GitHub SSO */}
					<div class="flex items-center gap-3 mb-4">
						<div class="flex-1 h-px bg-secondary-medium" />
						<span class="text-gray-500 text-xs">or</span>
						<div class="flex-1 h-px bg-secondary-medium" />
					</div>
					<Button
						variant={ButtonVariant.Plain}
						class="w-full py-3 mb-2 gap-3 rounded-xs bg-black! text-white! text-sm font-medium border border-white/25 enabled:hover:bg-[#1f1f1f]! enabled:hover:cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed transition-colors duration-200"
						type="button"
						loading={githubLoading}
						loadingContent={() => <span>Redirecting to GitHub...</span>}
						onClick={handleGithubSignIn}
					>
						<img src="/icons/github.svg" alt="" aria-hidden="true" height="20" width="20" class="invert" />
						Continue with GitHub
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

export const Route = createFileRoute("/_logged-out/sign-up")({
	component: SignUp,
});
