import { createFileRoute, Link, useNavigate } from "@tanstack/solid-router";
import { Alert, Button, useToast, Turnstile } from "~/components";
import Input, { InputType, InputEventT, PasswordInput } from "~/components/input";
import { ButtonVariant } from "~/utils/color";
import { createSignal, Show } from "solid-js";
import { LoginRequest, LoginResponse } from "~/bindings";
import { httpRequest } from "~/utils/http-request";
import { createAsyncAction, useAuthState } from "~/hooks";
import { USERNAME_VALIDITY_PATTERN, validatePassword } from "~/utils/validation";
import OtpInput from "~/components/otp-input";

interface InputFields {
	userId: string;
	password: string;
	mfaOtp: string;
}

const Login = () => {
	const [, setAuthState] = useAuthState();
	const navigate = useNavigate();
	const toast = useToast();
	const [showMfa, setShowMfa] = createSignal(false);
	const [mfaOtp, setMfaOtp] = createSignal("");
	const [turnstileToken, setTurnstileToken] = createSignal<string>(import.meta.env.VITE_TURNSTILE_SITE_KEY);
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

		// FIXME: Poor regex, improve this
		// if (!USERNAME_VALIDITY_REGEX.test(userId)) {
		//   setInputError((prev) => ({
		//     ...prev,
		//     userId: "Invalid Username format.",
		//   }));
		//   return false;
		// }

		if (password.length === 0) {
			setInputError((prev) => ({
				...prev,
				password: "Password cannot be empty.",
			}));
			return false;
		}

		const passwordValidation = validatePassword(password);
		if (!passwordValidation.valid) {
			setInputError((prev) => ({
				...prev,
				password: passwordValidation.error || "Invalid password.",
			}));
			return false;
		}

		if (!turnstileToken()) {
			toast("Please complete the security verification", "error");
			return false;
		}

		return true;
	};

	const { execute: submitLogin, isLoading } = createAsyncAction(async () => {
		const { userId, password } = inputs();
		if (!validateInputs()) return;

		const requestBody: LoginRequest = {
			userId,
			password,
			mfaOtp: showMfa() && mfaOtp() !== "" ? mfaOtp() : undefined,
			cfTurnstileToken: turnstileToken(),
		};

		const loginResp = await httpRequest<LoginResponse>("/api/auth/sign-in", {
			method: "POST",
			body: JSON.stringify(requestBody),
		});

		if (loginResp.ok) {
			console.log("Login successful");
			setAuthState({
				type: "LoggedIn",
				accessToken: loginResp.data.accessToken,
				refreshToken: loginResp.data.refreshToken,
			});
			navigate({ to: "/", replace: true });
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
			{/* Login Card */}
			<form
				method="post"
				onSubmit={async (e) => {
					e.preventDefault();
					await submitLogin().catch(() => {});
				}}
				class="bg-secondary p-12 rounded-sm shadow-2xl w-full max-w-128 relative z-10 border border-secondary-medium"
			>
				{/* Header */}
				<div class="mb-10 items-center justify-between flex flex-row">
					<h1 class="font-bold text-2xl text-white">Login</h1>
					<div class="flex flex-row items-end">
						<div class="text-gray-400 font-extralight text-sm mr-2">New User?</div>
						<Link class="text-primary font-thin text-sm hover:underline" to="/sign-up">
							Sign Up
						</Link>
					</div>
				</div>

				{/* Form */}
				<div>
					{/** TODO: add min max values for input */}
					<Input
						required={true}
						type={InputType.Text}
						placeholder="Username"
						pattern={USERNAME_VALIDITY_PATTERN}
						title="Username must start and end with an alphanumeric character and can contain underscores, dots, or hyphens in between."
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
						<OtpInput
							outerClass="mt-4"
							inputVariant="medium"
							otpDigits={() => mfaOtp().split("")}
							setOtpDigits={(digits) => setMfaOtp(digits.join(""))}
						/>
					</Show>

					{/* Turnstile Widget */}
					<div class="mt-6 flex justify-center">
						<Turnstile
							onVerify={setTurnstileToken}
							onExpire={() => setTurnstileToken("")}
							onError={() => setTurnstileToken("")}
							action="login"
						/>
					</div>

					{/* Login Button */}
					<div class="pt-8 w-full flex flex-row items-center justify-between">
						<Link to="/forgot-password" class="text-primary text-xs hover:underline font-light">
							Forgot password?
						</Link>
						<Button
							variant={ButtonVariant.Contained}
							class="py-4 text-base font-semibold px-xxl flex-end"
							type="submit"
							loading={isLoading}
							loadingContent={() => <span>Logging in...</span>}
						>
							Login
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

export const Route = createFileRoute("/_auth/login")({
	component: Login,
});
