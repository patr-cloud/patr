import { createFileRoute } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import { createMemo, ErrorBoundary, Show } from "solid-js";
import { LoadingSpinner, PageContainer, PageContainerBody } from "~/components";
import { useRoleInfoQuery, useWorkspaceInfoQuery } from "~/hooks/fetch";
import RoleHeader from "./-components/role-header";
import UsersAssignedToRole from "./-components/users";
import EditPermissions from "./-components/edit";

const RoleInfo = () => {
	const params = Route.useParams();
	const search = Route.useSearch();

	const activeTab = createMemo(() => (search().tab === "users" ? "users" : "permissions"));

	const workspaceInfoQuery = useWorkspaceInfoQuery();
	const roleInfoQuery = useRoleInfoQuery(() => params().roleId);

	return (
		<>
			<Title>Role Details | Patr</Title>
			<PageContainer>
				<RoleHeader
					roleName={roleInfoQuery.data?.name}
					workspaceName={workspaceInfoQuery.data?.name}
					activeTab={activeTab()}
				/>
				<PageContainerBody class="flex flex-col gap-6">
					<ErrorBoundary
						fallback={(err, reset) => (
							<div class="flex flex-col items-center justify-center gap-4 py-16">
								<p class="text-error text-sm">Error loading role: {err.message}</p>
								<button class="text-primary hover:underline text-sm" onClick={reset}>
									Retry
								</button>
							</div>
						)}
					>
						<Show
							when={!roleInfoQuery.isPending}
							fallback={
								<div class="flex items-center justify-center gap-2 py-16 text-grey">
									<LoadingSpinner size={20} />
									<span class="text-sm">Loading role information...</span>
								</div>
							}
						>
							<Show when={roleInfoQuery.data}>
								<div class="flex flex-col gap-4">
									<Show when={activeTab() === "permissions"}>
										<EditPermissions />
									</Show>

									<Show when={activeTab() === "users"}>
										<UsersAssignedToRole />
									</Show>
								</div>
							</Show>
						</Show>
					</ErrorBoundary>
				</PageContainerBody>
			</PageContainer>
		</>
	);
};

export const Route = createFileRoute("/_logged-in/_workspaced/workspace/roles/$roleId")({
	validateSearch: (search: Record<string, unknown>): { tab: string } => ({
		tab: (search.tab as string) || "",
	}),
	component: RoleInfo,
});
