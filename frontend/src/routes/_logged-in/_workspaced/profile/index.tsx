import { createFileRoute } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import { createResource, Suspense } from "solid-js";
import { GetUserInfoResponse } from "~/bindings";
import { LoadingSpinner, PageContainer, PageContainerBody, PageContainerHead, useToast } from "~/components";
import { useAuthState } from "~/hooks";
import { httpRequest } from "~/utils/http-request";
import UserSettingsInfoTab from "./-components/info";
import ChangePasswordTab from "./-components/change-password";

const UserSettingsPage = () => {
	const [authState] = useAuthState();
	const toast = useToast();

	const [userInfo, { mutate: mutateUserInfo, refetch: refetchUserInfo }] = createResource(
		authState(),
		async (auth) => {
			if (auth === null || auth.type !== "LoggedIn") {
				console.log("Auth is null or LoggedOut, returning null");
				return undefined;
			}

			try {
				const response = await httpRequest<GetUserInfoResponse>(`${import.meta.env.VITE_BASE_URL}/api/user`, {
					method: "GET",
				});

				if (!response.ok) {
					toast("Failed to fetch user info", "error");
					return undefined;
				}

				return response.data;
			} catch (error) {
				console.error("Failed to fetch user info:", error);
				return undefined;
			}
		}
	);

	return (
		<>
			<Title>Profile | Patr</Title>
			<PageContainer>
				<PageContainerHead
					breadcrumbs={[
						{
							label: "Account Settings",
						},
					]}
					subText="Manage your profile information and security preferences"
				/>
				<PageContainerBody class="flex flex-col gap-8">
					<Suspense
						fallback={
							<div class="flex items-center justify-center gap-2 py-16 text-grey">
								<LoadingSpinner size={20} />
								<span class="text-sm">Loading user info...</span>
							</div>
						}
					>
						<div class="flex flex-1 flex-col gap-6 items-start w-full">
							<UserSettingsInfoTab
								userInfo={userInfo}
								mutateUserInfo={mutateUserInfo}
								refetchUserInfo={refetchUserInfo}
							/>
							<ChangePasswordTab
								mutateUserInfo={mutateUserInfo}
								userInfo={userInfo}
								refetchUserInfo={refetchUserInfo}
							/>
						</div>
					</Suspense>
				</PageContainerBody>
			</PageContainer>
		</>
	);
};

export const Route = createFileRoute("/_logged-in/_workspaced/profile/")({
	validateSearch: (search: Record<string, unknown>): { tab: string } => ({
		tab: (search.tab as string) || "",
	}),
	component: UserSettingsPage,
});
