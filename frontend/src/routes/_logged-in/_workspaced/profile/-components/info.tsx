import { Resource, Setter } from "solid-js";
import { GetUserInfoResponse } from "~/bindings";
import { Button, ButtonVariant, Input, InputType, InputLabel, Modal, useToast } from "~/components";
import { useAuthState } from "~/hooks";
import TwoFactorAuthModal from "./two-fa";
import { httpRequest } from "~/utils/http-request";
import { EventT } from "~/utils/types";

interface UserSettingsInfoTabProps {
	userInfo: Resource<GetUserInfoResponse | undefined>;
	mutateUserInfo: Setter<GetUserInfoResponse | undefined>;
	refetchUserInfo: () => GetUserInfoResponse | Promise<GetUserInfoResponse | undefined> | null | undefined;
}

const UserSettingsInfoTab = (props: UserSettingsInfoTabProps) => {
	const [authState] = useAuthState();
	const toast = useToast();

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
					firstName: props.userInfo.latest?.firstName,
					lastName: props.userInfo.latest?.lastName,
				}),
			});

			if (!response.ok) {
				console.error("Failed to update user info:", response.data.error);
				toast("Failed to update user info", "error");
				return;
			}

			toast("User info updated successfully", "success");
			props.refetchUserInfo();
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
					value={props.userInfo.latest?.firstName || ""}
					class="flex-5"
					id="first-name"
					name="first-name"
					autocomplete="given-name"
					placeholder="First Name"
					type={InputType.Text}
					onInput={(e) => {
						props.mutateUserInfo((prev) => {
							return prev
								? {
										...prev,
										firstName: e.currentTarget.value,
									}
								: undefined;
						});
					}}
				/>
				<Input
					value={props.userInfo.latest?.lastName || ""}
					class="flex-5"
					id="last-name"
					name="last-name"
					autocomplete="family-name"
					placeholder="Last Name"
					type={InputType.Text}
					onInput={(e) => {
						props.mutateUserInfo((prev) => {
							return prev
								? {
										...prev,
										lastName: e.currentTarget.value,
									}
								: undefined;
						});
					}}
				/>
				<Button type="submit" variant={ButtonVariant.Contained}>
					Update
				</Button>
			</form>

			<div class="flex gap-4 items-center w-full">
				<InputLabel parentClass="flex-1" for="recovery-email" label="Email" />

				<Input
					value={props.userInfo.latest?.recoveryEmail || ""}
					class="flex-11"
					id="recovery-email"
					name="recovery-email"
					autocomplete="email"
					placeholder="Recovery Email"
					type={InputType.Text}
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
								{props.userInfo.latest?.isMfaEnabled ? "Disable" : "Enable"} 2FA Settings
							</Button>
						)}
						renderModalContent={(close) => (
							<TwoFactorAuthModal
								isMfaEnabled={!!props.userInfo.latest?.isMfaEnabled}
								refetchUserInfo={props.refetchUserInfo}
								closeFn={close}
							/>
						)}
					/>
				</div>
			</form>
		</>
	);
};

export default UserSettingsInfoTab;
