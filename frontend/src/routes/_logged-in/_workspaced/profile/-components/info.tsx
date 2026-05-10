import { createEffect, createSignal } from "solid-js";
import { GetUserInfoResponse } from "~/bindings";
import { Button, ButtonVariant, Input, InputType, InputLabel, Modal, useToast } from "~/components";
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
			<form onSubmit={onUpdateName} class="flex gap-4 items-center w-full">
				<InputLabel parentClass="flex-1" for="first-name" label="Name" />
				<Input
					value={localInfo()?.firstName || ""}
					class="flex-5"
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
					class="flex-5"
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
			</form>

			<div class="flex gap-4 items-center w-full">
				<InputLabel parentClass="flex-1" for="email" label="Email" />

				<Input
					value={localInfo()?.email || ""}
					class="flex-11"
					id="email"
					name="email"
					autocomplete="email"
					placeholder="Email"
					type={InputType.Email}
					disabled
				/>
			</div>

			<form class="flex gap-4 items-start w-full">
				<InputLabel parentClass="flex-1" label="Two-Factor Authentication" />

				<div class="flex-11">
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
			</form>
		</>
	);
};

export default UserSettingsInfoSection;
