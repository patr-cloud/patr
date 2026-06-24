import { Show, createEffect, createSignal } from "solid-js";
import { Alert, Button, ButtonVariant, Input, InputType, InputWithLabel, Modal, useToast } from "~/components";
import { useAuthState } from "~/hooks";
import { useUserInfoQuery } from "~/hooks/fetch";
import { userInfoKeys } from "~/hooks/query-keys";
import { useQueryClient } from "@tanstack/solid-query";
import TwoFactorAuthModal from "./two-fa";
import { httpRequest } from "~/utils/http-request";
import { EventT } from "~/utils/types";
import { validateNameField } from "~/utils/validation";

const UserSettingsInfoSection = () => {
	const [authState] = useAuthState();
	const toast = useToast();
	const queryClient = useQueryClient();

	const userInfoQuery = useUserInfoQuery();

	// Editable form drafts. Seeded from the query once on first load, then
	// untouched by background updates — the user's typing is the source of
	// truth for these until they hit Update. Non-editable fields (email,
	// MFA status) read from `userInfoQuery.data` directly.
	const [firstName, setFirstName] = createSignal<string | undefined>();
	const [lastName, setLastName] = createSignal<string | undefined>();
	const [firstNameError, setFirstNameError] = createSignal<string | undefined>(undefined);
	const [lastNameError, setLastNameError] = createSignal<string | undefined>(undefined);
	const [submitting, setSubmitting] = createSignal(false);

	createEffect(() => {
		const info = userInfoQuery.data;
		if (!info) return;
		if (firstName() === undefined) setFirstName(info.firstName ?? "");
		if (lastName() === undefined) setLastName(info.lastName ?? "");
	});

	const refetchUserInfo = () => queryClient.invalidateQueries({ queryKey: userInfoKeys.current() });

	const onUpdateName = async (e: EventT<SubmitEvent, HTMLFormElement>) => {
		e.preventDefault();
		const auth = authState();

		if (!auth || auth.type !== "LoggedIn") {
			toast("You must be logged in to update your name", "error");
			return;
		}

		const fnErr = validateNameField(firstName() ?? "");
		const lnErr = validateNameField(lastName() ?? "");
		setFirstNameError(fnErr);
		setLastNameError(lnErr);
		if (fnErr || lnErr) return;

		// Prevent overlapping submits: a second submit while one is in flight
		// would fire two concurrent PATCHes with no commit-ordering guarantee.
		if (submitting()) return;
		setSubmitting(true);

		try {
			const response = await httpRequest(`${import.meta.env.VITE_BASE_URL}/api/user`, {
				method: "PATCH",
				body: JSON.stringify({
					firstName: firstName(),
					lastName: lastName(),
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
		} finally {
			setSubmitting(false);
		}
	};
	return (
		<>
			<form onSubmit={onUpdateName} class="w-full">
				<InputWithLabel for="first-name" label="Name">
					<div class="flex flex-col md:flex-row gap-2 w-full">
						<Input
							value={firstName() ?? ""}
							class="md:flex-1"
							id="first-name"
							name="first-name"
							autocomplete="given-name"
							placeholder="First Name"
							type={InputType.Text}
							onInput={(e) => {
								setFirstName(e.currentTarget.value);
								setFirstNameError(undefined);
							}}
						/>
						<Input
							value={lastName() ?? ""}
							class="md:flex-1"
							id="last-name"
							name="last-name"
							autocomplete="family-name"
							placeholder="Last Name"
							type={InputType.Text}
							onInput={(e) => {
								setLastName(e.currentTarget.value);
								setLastNameError(undefined);
							}}
						/>
						<Button type="submit" variant={ButtonVariant.Contained} disabled={submitting()}>
							Update
						</Button>
					</div>
					<Show when={firstNameError()}>
						<Alert message={firstNameError()!} type="error" />
					</Show>
					<Show when={lastNameError()}>
						<Alert message={lastNameError()!} type="error" />
					</Show>
				</InputWithLabel>
			</form>

			<InputWithLabel for="recovery-email" label="Email">
				<Input
					value={userInfoQuery.data?.recoveryEmail || ""}
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
								{userInfoQuery.data?.isMfaEnabled ? "Disable" : "Enable"} 2FA Settings
							</Button>
						)}
						renderModalContent={(close) => (
							<TwoFactorAuthModal
								isMfaEnabled={!!userInfoQuery.data?.isMfaEnabled}
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
