import { createResource, Suspense } from "solid-js";
import { useSearchParams } from "@solidjs/router";
import { GetUserInfoResponse } from "~/bindings";
import { PageContainer, PageContainerBody, PageContainerHead, useToast, HeadTab } from "~/components";
import { useAuthState } from "~/hooks";
import { httpRequest } from "~/utils/http-request";
import { EventT } from "~/utils/types";
import UserSettingsInfoTab from "~/pages/user/settings/info";
import ChangePasswordTab from "./change-password";

const UserSettingsPage = () => {
	const [authState] = useAuthState();
	const toast = useToast();

	const [searchParams, setSearchParams] = useSearchParams();
	const tab = () => (searchParams.tab as string) || "";

	const [userInfo, { mutate: mutateUserInfo, refetch: refetchUserInfo }] = createResource(authState(), async (auth) => {
		if (auth === null || auth.type !== "LoggedIn") {
			console.log("Auth is null or LoggedOut, returning null");
			return undefined;
		}

		try {
			const response = await httpRequest<GetUserInfoResponse>(`${import.meta.env.VITE_BASE_URL}/api/user`, {
				method: "GET",
				headers: {
					Authorization: `Bearer ${auth.accessToken}`,
				},
			});

			if (!response.ok) {
				console.error("Failed to fetch workspaces:", response.data.error);
				toast("Failed to fetch workspaces", "error");
				return undefined;
			}

			return response.data;
		} catch (error) {
			console.error("Failed to fetch user info:", error);
			return undefined;
		}
	});

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
				headers: {
					Authorization: `Bearer ${auth.accessToken}`,
				},
				body: JSON.stringify({
					firstName: userInfo.latest?.firstName,
					lastName: userInfo.latest?.lastName,
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
		<PageContainer>
			<PageContainerHead
				breadcrumbs={[
					{
						label: "Account Settings",
					},
				]}
				subText="Manage your profile information and security preferences"
				bottomContent={() => (
					<HeadTab
						tab={tab}
						searchParams={searchParams}
						setSearchParams={setSearchParams}
						tabItems={[
							{
								label: "Info",
								value: "",
								onClick: (value) => setSearchParams({ tab: value }),
							},
						]}
					/>
				)}
			/>
			<PageContainerBody class="flex flex-col gap-8">
				<Suspense fallback={<div>Loading user info...</div>}>
					<div class="flex flex-1 flex-col gap-6 items-start w-full">
						<UserSettingsInfoTab
							userInfo={userInfo}
							mutateUserInfo={mutateUserInfo}
							refetchUserInfo={refetchUserInfo}
						/>
						<ChangePasswordTab mutateUserInfo={mutateUserInfo} userInfo={userInfo} refetchUserInfo={refetchUserInfo} />
					</div>
				</Suspense>
			</PageContainerBody>
		</PageContainer>
	);
};

export default UserSettingsPage;
