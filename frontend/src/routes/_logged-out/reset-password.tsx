import { createFileRoute, Link, useNavigate } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import { createSignal, onMount, Show } from "solid-js";
import { ResetPasswordRequest } from "~/bindings";
import {
	Alert,
	Button,
	ButtonVariant,
	Input,
	InputType,
	OtpInput,
	PasswordInput,
	PasswordStrength,
	Turnstile,
	useToast,
} from "~/components";
import { createAsyncAction } from "~/hooks";
import { httpRequest } from "~/utils/http-request";
import { validatePassword } from "~/utils/validation";

const ResetPassword = () => {
	const navigate = useNavigate();
	const toast = useToast();

	const search = Route.useSearch();
	const initialUserId = search().userId || "";
	const initialOtp = search().otp || "";

	const [userId, setUserId] = createSignal(initialUserId);
	const [userIdError, setUserIdError] = createSignal("");
	const [otpDigits, setOtpDigits] = createSignal<string[]>(
		initialOtp
			? [...initialOtp.replace(/\D/g, "").slice(0, 6).padEnd(6, " ")].map((c) => (c === " " ? "" : c))
			: ["", "", "", "", "", ""]
	);
	const [newPassword, setNewPassword] = createSignal("");
	const [newPasswordError, setNewPasswordError] = createSignal("");
	const [confirmPassword, setConfirmPassword] = createSignal("");
	const [confirmPasswordError, setConfirmPasswordError] = createSignal("");
	const [turnstileToken, setTurnstileToken] = createSignal<string>("");

	const [showPasswordStrength, setShowPasswordStrength] = createSignal(false);
	const [passwordAnchor, setPasswordAnchor] = createSignal<HTMLDivElement>();

	// Strip query params after reading them so the OTP doesn't linger in the URL.
	onMount(() => {
		if (search().userId || search().otp) {
			navigate({
				to: "/reset-password",
				search: { userId: undefined, otp: undefined },
				replace: true,
			});
		}
	});

	const { execute: handleSubmit, isLoading } = createAsyncAction(async () => {
		// Re-validate at submit time so users get inline feedback for fields the
		// disabled-button gate doesn't cover (userId, password rules, mismatch).
		if (!userId().trim()) {
			setUserIdError("Username or email is required.");
			return;
		}

		const pwd = newPassword();
		const pwdCheck = validatePassword(pwd);
		if (!pwdCheck.valid) {
			setNewPasswordError(pwdCheck.error ?? "Password is invalid");
			return;
		}

		if (pwd !== confirmPassword()) {
			setConfirmPasswordError("Passwords do not match.");
			return;
		}

		if (!turnstileToken()) {
			toast("Please complete the security verification", "error");
			return;
		}

		const body: ResetPasswordRequest = {
			userId: userId().trim(),
			password: pwd,
			verificationToken: otpDigits().join(""),
			cfTurnstileToken: turnstileToken(),
		};

		const resp = await httpRequest("/api/auth/reset-password", {
			method: "POST",
			body: JSON.stringify(body),
		});

		if (resp.ok) {
			toast("Password reset. You can now log in.", "success");
			navigate({ to: "/login" });
		} else {
			// Mirror the generic-error stance the backend takes — don't leak
			// whether the userId existed or the OTP was wrong.
			toast("Invalid or expired reset link. Please request a new one.", "error");
		}
	});

	const onSubmit = async (e: Event) => {
		e.preventDefault();
		await handleSubmit().catch(() => {
			toast("An unexpected error occurred. Please try again.", "error");
		});
	};

	return (
		<>
			<Title>Reset Password | Patr</Title>
			<form
				noValidate
				onSubmit={onSubmit}
				class="bg-secondary p-12 rounded-sm shadow-2xl w-full max-w-128 relative z-10 border border-secondary-medium"
			>
				{/* Header */}
				<div class="mb-10 items-center justify-between flex flex-row">
					<h1 class="font-bold text-2xl text-white">Reset Password</h1>
					<div class="flex flex-row items-baseline">
						<div class="text-gray-400 font-extralight text-sm mr-2">Remembered it?</div>
						<Link class="text-primary font-thin text-sm hover:underline" to="/login">
							Login
						</Link>
					</div>
				</div>

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

				<div class="mt-6">
					<p class="text-grey text-sm mb-3">Enter the 6-digit verification code sent to your email</p>
					<OtpInput inputVariant="medium" otpDigits={otpDigits} setOtpDigits={setOtpDigits} />
				</div>

				<div
					ref={setPasswordAnchor}
					class="mt-6"
					onFocusIn={() => setShowPasswordStrength(true)}
					onFocusOut={(e) => {
						if (!e.currentTarget.contains(e.relatedTarget as Node | null)) {
							setShowPasswordStrength(false);
						}
					}}
				>
					<PasswordInput
						placeholder="New password"
						autocomplete="new-password"
						required={true}
						name="new-password"
						id="new-password"
						value={newPassword}
						onInput={(e) => {
							setNewPassword((e.currentTarget as HTMLInputElement).value);
							setNewPasswordError("");
						}}
						styleVariant="medium"
					/>
				</div>
				<PasswordStrength password={newPassword} anchor={passwordAnchor} show={showPasswordStrength} />
				<Show when={newPasswordError()}>
					<div class="mt-1">
						<Alert message={newPasswordError()} type="error" />
					</div>
				</Show>

				<PasswordInput
					placeholder="Confirm new password"
					autocomplete="new-password"
					required={true}
					name="confirm-password"
					id="confirm-password"
					value={confirmPassword}
					onInput={(e) => {
						setConfirmPassword((e.currentTarget as HTMLInputElement).value);
						setConfirmPasswordError("");
					}}
					class="mt-4"
					styleVariant="medium"
				/>
				<Show when={confirmPasswordError()}>
					<div class="mt-1">
						<Alert message={confirmPasswordError()} type="error" />
					</div>
				</Show>

				{/* Turnstile Widget */}
				<div class="mt-6 flex justify-center">
					<Turnstile
						onVerify={setTurnstileToken}
						onExpire={() => setTurnstileToken("")}
						onError={() => setTurnstileToken("")}
						action="reset-password"
					/>
				</div>

				{/* Submit */}
				<div class="pt-8 w-full flex flex-row items-center justify-between">
					<Link to="/forgot-password" class="text-primary text-xs hover:underline font-light">
						Request a new code
					</Link>
					<Button
						loading={isLoading}
						loadingContent={() => <span>Resetting...</span>}
						variant={ButtonVariant.Contained}
						class="py-4 text-base font-semibold px-8"
						type="submit"
						disabled={!turnstileToken() || otpDigits().some((d) => d === "")}
					>
						Reset Password
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

export const Route = createFileRoute("/_logged-out/reset-password")({
	validateSearch: (search: Record<string, unknown>): { userId?: string; otp?: string } => ({
		userId: (search.userId as string) || undefined,
		otp: (search.otp as string) || undefined,
	}),
	component: ResetPassword,
});
