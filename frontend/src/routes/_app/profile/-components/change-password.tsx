import { createSignal, Resource, Setter, Show } from "solid-js";
import { GetUserInfoResponse } from "~/bindings";
import { Alert, Button, ButtonVariant, useToast } from "~/components";
import { PasswordInput } from "~/components/input";
import InputLabel from "~/components/input-label";
import { EventT } from "~/utils/types";
import { ChangePasswordRequest, ChangePasswordResponse } from "~/bindings";
import { useAuthState } from "~/hooks";
import { httpRequest } from "~/utils/http-request";
import OtpInput from "~/components/otp-input";

interface UserSettingsInfoTabProps {
	userInfo: Resource<GetUserInfoResponse | undefined>;
	mutateUserInfo: Setter<GetUserInfoResponse | undefined>;
	refetchUserInfo: () => GetUserInfoResponse | Promise<GetUserInfoResponse | undefined> | null | undefined;
}

const ChangePasswordTab = (_: UserSettingsInfoTabProps) => {
	const [authState] = useAuthState();
	const toast = useToast();

	const [oldPassword, setOldPassword] = createSignal("");
	const [newPassword, setNewPassword] = createSignal("");
	const [confirmPassword, setConfirmPassword] = createSignal("");

	const [showMfa, setShowMfa] = createSignal(false);
	const [mfaOtp, setMfaOtp] = createSignal("");

	const [inputError, setInputError] = createSignal({
		oldPassword: "",
		newPassword: "",
		confirmPassword: "",
		mfaOtp: "",
		error: "",
	});

	const onUpdatePassword = async (e: EventT<SubmitEvent, HTMLFormElement>) => {
		e.preventDefault();

		const auth = authState();
		if (!auth || auth.type !== "LoggedIn") {
			toast("You must be logged in to update your password", "error");
			return;
		}

		try {
			const body: ChangePasswordRequest = {
				currentPassword: oldPassword(),
				newPassword: newPassword(),
			};

			if (showMfa()) {
				body.mfaOtp = mfaOtp();
			}

			const response = await httpRequest<ChangePasswordResponse>(
				`${import.meta.env.VITE_BASE_URL}/api/user/change-password`,
				{
					method: "POST",
					body: JSON.stringify(body),
				}
			);

			if (!response.ok) {
				console.error("Failed to change password:", response.data.error);
				switch (response.data.error) {
					case "mfaRequired":
						setShowMfa(true);
						break;
					case "invalidPassword":
						toast("Current password is incorrect", "error");
						break;
					default:
						toast("Failed to change password", "error");
						setInputError((prev) => ({
							...prev,
							error: response.data.message,
						}));
				}
				return;
			}

			toast("Password updated successfully", "success");
			// Clear input fields
			setOldPassword("");
			setNewPassword("");
			setConfirmPassword("");
			setMfaOtp("");
			setShowMfa(false);
		} catch (error) {
			console.error("Failed to change password:", error);
			toast("Failed to change password", "error");
		}
	};

	return (
		<div class="flex flex-col gap-4 w-full">
			<div class="flex items-center">
				<p class="text-xl text-white">Change Password</p>
			</div>
			<form
				onSubmit={onUpdatePassword}
				class="flex flex-col gap-4 items-center justify-between w-full h-full flex-1"
			>
				<div class="flex flex-col justify-between items-start gap-4 w-full">
					<div class="flex gap-4 items-center w-full">
						<InputLabel parentClass="flex-[1.25]" for="current-password" label="Current Password" />

						<div class="flex-[10.75]">
							<PasswordInput
								value={oldPassword()}
								name="current-password"
								placeholder="Current Password"
								onInput={(e) => setOldPassword(e.currentTarget.value)}
							/>

							<Show when={inputError().oldPassword}>
								<div class="flex justify-start items-center mt-1">
									<Alert message={inputError().oldPassword} type="error" />
								</div>
							</Show>
						</div>
					</div>

					<div class="flex gap-4 items-center w-full">
						<InputLabel parentClass="flex-[1.25]" for="new-password" label="New Password" />

						<div class="flex-[10.75]">
							<PasswordInput
								value={newPassword()}
								name="new-password"
								placeholder="New Password"
								onInput={(e) => setNewPassword(e.currentTarget.value)}
							/>

							<Show when={inputError().newPassword}>
								<div class="flex justify-start items-center mt-1">
									<Alert message={inputError().newPassword} type="error" />
								</div>
							</Show>
						</div>
					</div>

					<div class="flex gap-4 items-center w-full">
						<InputLabel parentClass="flex-[1.25]" for="confirm-password" label="Confirm Password" />
						<div class="flex-[10.75]">
							<PasswordInput
								value={confirmPassword()}
								name="confirm-password"
								placeholder="Confirm Password"
								onInput={(e) => setConfirmPassword(e.currentTarget.value)}
							/>

							<Show when={inputError().confirmPassword}>
								<div class="flex justify-start items-center mt-1">
									<Alert message={inputError().confirmPassword} type="error" />
								</div>
							</Show>
						</div>
					</div>

					<Show when={showMfa()}>
						<div class="flex gap-4 items-center w-full">
							<InputLabel parentClass="flex-[1.25]" for="mfa-otp" label="MFA OTP" />
							<div class="flex-[10.75]">
								<OtpInput
									outerClass="w-1/3"
									inputVariant="medium"
									otpDigits={() => mfaOtp().split("")}
									setOtpDigits={(digits) => setMfaOtp(digits.join(""))}
								/>
							</div>
						</div>
					</Show>

					<Show when={inputError().error}>
						<div class="flex justify-start items-center mt-1">
							<Alert message={inputError().error} type="error" />
						</div>
					</Show>
				</div>

				<div class="flex items-center justify-end w-full">
					<Button
						disabled={!oldPassword() || !newPassword() || newPassword() !== confirmPassword()}
						type="submit"
						variant={ButtonVariant.Contained}
					>
						Update Password
					</Button>
				</div>
			</form>
		</div>
	);
};

export default ChangePasswordTab;
