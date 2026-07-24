import { createFileRoute } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import { lazy, Show } from "solid-js";
import { LoadingSpinner, PageContainer, PageContainerBody, PageContainerHead } from "~/components";
import { useUserInfoQuery } from "~/hooks/fetch";
import { IS_CLOUD } from "~/utils/env";
import UserSettingsInfoSection from "./-components/info";
import ChangePasswordSection from "./-components/change-password";

const ConnectedAccountsSection = IS_CLOUD ? lazy(() => import("./-components/connected-accounts")) : null;

const UserSettingsPage = () => {
	const userInfoQuery = useUserInfoQuery();

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
					<Show
						when={!userInfoQuery.isPending}
						fallback={
							<div class="flex items-center justify-center gap-2 py-16 text-grey">
								<LoadingSpinner size={20} />
								<span class="text-sm">Loading user info...</span>
							</div>
						}
					>
						<div class="flex flex-1 flex-col gap-6 items-start w-full">
							<UserSettingsInfoSection />
							<ChangePasswordSection />
							{ConnectedAccountsSection && <ConnectedAccountsSection />}
						</div>
					</Show>
				</PageContainerBody>
			</PageContainer>
		</>
	);
};

export const Route = createFileRoute("/_logged-in/profile/")({
	component: UserSettingsPage,
});
