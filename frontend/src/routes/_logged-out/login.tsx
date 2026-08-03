import { createFileRoute, Link, useNavigate, useRouter } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import {
	Alert,
	Button,
	Input,
	InputType,
	type InputEventT,
	PasswordInput,
	OtpInput,
	useToast,
	Turnstile,
} from "~/components";
import { ButtonVariant } from "~/utils/color";
import { createSignal, Show } from "solid-js";
import { SocialLoginInitiateResponse, LoginRequest, LoginResponse } from "~/bindings";
import { httpRequest } from "~/utils/http-request";
import { createAsyncAction, useAuthState } from "~/hooks";
import { IS_CLOUD } from "~/utils/env";
import { USERNAME_OR_EMAIL_PATTERN, validateUsernameOrEmail } from "~/utils/validation";

interface InputFields {
	userId: string;
	password: string;
	mfaOtp: string;
}

const Login = () => {
	const [, setAuthState] = useAuthState();
	const router = useRouter();
	const navigate = useNavigate();
	const toast = useToast();
	const [githubLoading, setGithubLoading] = createSignal(false);
	const [showMfa, setShowMfa] = createSignal(false);
	const [mfaOtp, setMfaOtp] = createSignal("");
	const [turnstileToken, setTurnstileToken] = createSignal<string>("");
	const [inputs, setInputs] = createSignal<InputFields>({
		userId: "",
		password: "",
		mfaOtp: "",
	});
	const [inputError, setInputError] = createSignal<InputFields>({
		userId: "",
		password: "",
		mfaOtp: "",
	});

	const handleInput = (e: InputEventT) => {
		e.preventDefault();

		const { id, value } = e.currentTarget;

		setInputs((prev) => ({ ...prev, [id]: value }));
		setInputError((prev) => ({ ...prev, [id]: "" }));
	};

	const validateInputs = (): boolean => {
		const { userId, password } = inputs();

		const userIdError = validateUsernameOrEmail(userId);
		if (userIdError) {
			setInputError((prev) => ({ ...prev, userId: userIdError }));
			return false;
		}

		if (password.length === 0) {
			setInputError((prev) => ({
				...prev,
				password: "Password cannot be empty.",
			}));
			return false;
		}

		if (IS_CLOUD && !turnstileToken()) {
			toast("Please complete the security verification", "error");
			return false;
		}

		return true;
	};

	const handleGithubSignIn = IS_CLOUD
		? async () => {
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
			}
		: undefined;

	const { execute: submitLogin, isLoading } = createAsyncAction(async () => {
		const { userId, password } = inputs();
		if (!validateInputs()) return;

		const requestBody: LoginRequest = {
			userId,
			password,
			mfaOtp: showMfa() && mfaOtp() !== "" ? mfaOtp() : undefined,
			cfTurnstileToken: IS_CLOUD ? turnstileToken() : "self-hosted",
		};

		const loginResp = await httpRequest<LoginResponse>("/api/auth/sign-in", {
			method: "POST",
			body: JSON.stringify(requestBody),
		});

		if (loginResp.ok) {
			console.log("Login successful");
			const newAuth = {
				type: "LoggedIn" as const,
				accessToken: loginResp.data.accessToken,
				refreshToken: loginResp.data.refreshToken,
			};
			setAuthState(newAuth);
			router.update({
				...router.options,
				context: {
					...router.options.context,
					auth: newAuth,
				},
			});
			await router.invalidate();
			// If the user arrived here from a workspace invite link, resume that
			// flow instead of dropping them on the dashboard.
			if (sessionStorage.getItem("pendingWorkspaceInvite")) {
				navigate({ to: "/accept-invite", replace: true });
			} else {
				navigate({ to: "/", replace: true });
			}
		} else {
			console.error("Error during login:", loginResp);
			switch (loginResp.data.error) {
				case "invalidPassword":
					setInputError((prev) => ({
						...prev,
						password: "Incorrect password. Please try again.",
					}));
					break;
				case "userNotFound":
				case "invalidEmail":
					setInputError((prev) => ({
						...prev,
						userId: "User not found. Please check your username.",
					}));
					break;
				case "mfaRequired":
					setShowMfa(true);
					break;
				default:
					toast("Error during login: " + loginResp.statusText, "error");
					break;
			}
		}
	});

	return (
		<>
			<Title>Login | Patr</Title>
			{/* Login Card */}
			<form
				method="post"
				noValidate
				onSubmit={async (e) => {
					e.preventDefault();
					await submitLogin().catch(() => {
						toast("An unexpected error occurred. Please try again.", "error");
					});
				}}
				class="bg-secondary p-12 rounded-sm shadow-2xl w-full max-w-128 relative z-10 border border-secondary-medium"
			>
				{/* Header */}
				<div class="mb-10 items-center justify-between flex flex-row">
					<h1 class="font-bold text-2xl text-white">Login</h1>
					<Show when={IS_CLOUD}>
						<div class="flex flex-row items-end">
							<div class="text-gray-400 font-extralight text-sm mr-2">New User?</div>
							<Link class="text-primary font-thin text-sm hover:underline" to="/sign-up">
								Sign Up
							</Link>
						</div>
					</Show>
				</div>

				{/* Form */}
				<div>
					{/** TODO: add min max values for input */}
					<Input
						required={true}
						type={InputType.Text}
						placeholder="Username or Email"
						autocomplete="username"
						pattern={USERNAME_OR_EMAIL_PATTERN}
						title="Enter your username or the email you signed up with."
						id="userId"
						name="userId"
						class="mt-4"
						styleVariant="medium"
						value={inputs().userId}
						onInput={handleInput}
					/>
					<Show when={inputError().userId}>
						<div class="flex justify-start items-center mt-1">
							<Alert message={inputError().userId} type="error" />
						</div>
					</Show>

					<PasswordInput
						required={true}
						placeholder="Password"
						autocomplete="current-password"
						id="password"
						name="password"
						class="mt-4"
						styleVariant="medium"
						value={inputs().password}
						onInput={handleInput}
					/>
					<Show when={inputError().password}>
						<div class="flex justify-start items-center mt-1">
							<Alert message={inputError().password} type="error" />
						</div>
					</Show>

					<Show when={showMfa()}>
						<p class="mt-4 text-sm text-grey">Enter the 6-digit code from your authenticator app</p>
						<OtpInput
							outerClass="mt-4"
							inputVariant="medium"
							otpDigits={() => mfaOtp().split("")}
							setOtpDigits={(digits) => setMfaOtp(digits.join(""))}
						/>
					</Show>

					{/* Turnstile Widget */}
					{IS_CLOUD && (
						<div class="mt-6 flex justify-center">
							<Turnstile
								onVerify={setTurnstileToken}
								onExpire={() => setTurnstileToken("")}
								onError={() => setTurnstileToken("")}
								action="login"
							/>
						</div>
					)}

					{/* Login Button */}
					<div class="pt-8 w-full flex flex-row items-center justify-between">
						<Link to="/forgot-password" class="text-primary text-xs hover:underline font-light">
							Forgot password?
						</Link>
						<Button
							variant={ButtonVariant.Contained}
							class="py-4 text-base font-semibold px-8"
							type="submit"
							loading={isLoading}
							loadingContent={() => <span>Logging in...</span>}
							disabled={IS_CLOUD && !turnstileToken()}
						>
							Login
						</Button>
					</div>
				</div>

				{/* GitHub SSO */}
				{IS_CLOUD && (
					<>
						<div class="flex items-center gap-3 my-4">
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
							<img
								src="/icons/github.svg"
								alt=""
								aria-hidden="true"
								height="20"
								width="20"
								class="invert"
							/>
							Continue with GitHub
						</Button>
					</>
				)}
			</form>

			{/* Footer */}
			<div class="absolute bottom-6 left-0 right-0 text-center">
				<p class="text-gray-500 text-xs">&copy; {new Date().getFullYear()} Patr. All rights reserved.</p>
			</div>
		</>
	);
};

export const Route = createFileRoute("/_logged-out/login")({
	component: Login,
});
