import { createEffect, createSignal } from "solid-js";
import { GetUserInfoResponse } from "~/bindings";
import { Button, ButtonVariant, Input, InputType, InputWithLabel, Modal, useToast } from "~/components";
import { useAuthState } from "~/hooks";
import { useUserInfoQuery } from "~/hooks/fetch";
import { userInfoKeys } from "~/hooks/query-keys";
import { useQueryClient } from "@tanstack/solid-query";
import TwoFactorAuthModal from "./two-fa";
import { httpRequest } from "~/utils/http-request";
import { EventT } from "~/utils/types";

const UserSettingsInfoSection = () => {
	const [authState] = useAuthState();
	const toast = useToast();
	const queryClient = useQueryClient();

	const userInfoQuery = useUserInfoQuery();

	// Local state for form editing
	const [localInfo, setLocalInfo] = createSignal<GetUserInfoResponse | undefined>(undefined);

	createEffect(() => {
		if (userInfoQuery.data && !localInfo()) {
			setLocalInfo(userInfoQuery.data);
		}
	});

	const refetchUserInfo = () => {
		queryClient.invalidateQueries({ queryKey: userInfoKeys.current() });
	};

	const onUpdateName = async (e: EventT<SubmitEvent, HTMLFormElement>) => {
		e.preventDefault();
		const auth = authState();

		if (!auth || auth.type !== "LoggedIn") {
			toast("You must be logged in to update your name", "error");
			return;
		}

		try {
			const response = await httpRequest(`${import.meta.env.VITE_BASE_URL}/api/user`, {
				method: "PATCH",
				body: JSON.stringify({
					firstName: localInfo()?.firstName,
					lastName: localInfo()?.lastName,
				}),
			});

			if (!response.ok) {
				console.error("Failed to update user info:", response.data.error);
				toast("Failed to update user info", "error");
				return;
			}

			toast("User info updated successfully", "success");
			refetchUserInfo();
		} catch (error) {
			console.error("Failed to update user info:", error);
			toast("Failed to update user info", "error");
		}
	};
	return (
		<>
			<form onSubmit={onUpdateName} class="w-full">
				<InputWithLabel for="first-name" label="Name">
					<div class="flex flex-col md:flex-row gap-2 w-full">
						<Input
							value={localInfo()?.firstName || ""}
							class="md:flex-1"
							id="first-name"
							name="first-name"
							autocomplete="given-name"
							placeholder="First Name"
							type={InputType.Text}
							onInput={(e) => {
								setLocalInfo((prev: GetUserInfoResponse | undefined) =>
									prev ? { ...prev, firstName: e.currentTarget.value } : undefined
								);
							}}
						/>
						<Input
							value={localInfo()?.lastName || ""}
							class="md:flex-1"
							id="last-name"
							name="last-name"
							autocomplete="family-name"
							placeholder="Last Name"
							type={InputType.Text}
							onInput={(e) => {
								setLocalInfo((prev: GetUserInfoResponse | undefined) =>
									prev ? { ...prev, lastName: e.currentTarget.value } : undefined
								);
							}}
						/>
						<Button type="submit" variant={ButtonVariant.Contained}>
							Update
						</Button>
					</div>
				</InputWithLabel>
			</form>

			<InputWithLabel for="recovery-email" label="Email">
				<Input
					value={localInfo()?.recoveryEmail || ""}
					id="recovery-email"
					name="recovery-email"
					autocomplete="email"
					placeholder="Recovery Email"
					type={InputType.Text}
					disabled
				/>
			</InputWithLabel>

			<InputWithLabel label="Two-Factor Authentication">
				<div>
					<Modal
						renderTrigger={(open) => (
							<Button
								variant={ButtonVariant.Contained}
								type="button"
								class="text-primary"
								onClick={() => open(true)}
							>
								{localInfo()?.isMfaEnabled ? "Disable" : "Enable"} 2FA Settings
							</Button>
						)}
						renderModalContent={(close) => (
							<TwoFactorAuthModal
								isMfaEnabled={!!localInfo()?.isMfaEnabled}
								refetchUserInfo={refetchUserInfo}
								closeFn={close}
							/>
						)}
					/>
				</div>
			</InputWithLabel>
		</>
	);
};

export default UserSettingsInfoSection;
