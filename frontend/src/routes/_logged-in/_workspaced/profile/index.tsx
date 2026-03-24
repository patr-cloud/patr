import { createFileRoute } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import { createResource, Suspense } from "solid-js";
import { useNavigate } from "@tanstack/solid-router";
import { GetUserInfoResponse } from "~/bindings";
import { PageContainer, PageContainerBody, PageContainerHead, useToast, HeadTab } from "~/components";
import { useAuthState } from "~/hooks";
import { httpRequest } from "~/utils/http-request";
import UserSettingsInfoTab from "./-components/info";
import ChangePasswordTab from "./-components/change-password";

const UserSettingsPage = () => {
	const [authState] = useAuthState();
	const toast = useToast();

	const search = Route.useSearch();
	const navigate = useNavigate();
	const tab = () => search().tab;

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
					console.error("Failed to fetch workspaces:", response.data.error);
					toast("Failed to fetch workspaces", "error");
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
					bottomContent={() => (
						<HeadTab
							tab={tab}
							tabItems={[
								{
									label: "Info",
									value: "",
									onClick: (value) => navigate({ to: "/profile", search: { tab: value } }),
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
