import { createFileRoute } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import { createMemo, createResource, ErrorBoundary, Show, Suspense } from "solid-js";
import { PageContainer, PageContainerBody, useToast } from "~/components";
import { useAuthState } from "~/hooks";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { GetRoleInfoResponse } from "~/bindings/GetRoleInfoResponse";
import { GetWorkspaceInfoResponse } from "~/bindings/GetWorkspaceInfoResponse";
import { httpRequest } from "~/utils/http-request";
import RoleHeader from "./-components/role-header";
import UsersAssignedToRole from "./-components/users";
import EditPermissions from "./-components/edit";

const RoleInfo = () => {
	const params = Route.useParams();
	const search = Route.useSearch();
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();
	const toast = useToast();

	const activeTab = createMemo(() => (search().tab === "users" ? "users" : "permissions"));

	const fetchParams = createMemo(() => {
		return [authState(), workspaceId(), params().roleId] as const;
	});

	const [workspaceInfo] = createResource(
		() => [authState(), workspaceId()] as const,
		async ([auth, workspaceId]) => {
			if (!auth || auth.type !== "LoggedIn" || !workspaceId) {
				return;
			}

			const response = await httpRequest<GetWorkspaceInfoResponse>(
				`${import.meta.env.VITE_BASE_URL}/api/workspace/${workspaceId}`,
				{
					method: "GET",
				}
			);
			console.log("Workspace info response:", response.data);
			if (!response.ok) {
				console.error("Failed to fetch workspace info:", response.data.error);
				toast("Failed to fetch workspace info", "error");
				return;
			}

			return response.data;
		}
	);

	const [roleInfo, { refetch: refetchRoleInfo }] = createResource(
		fetchParams,
		async ([auth, workspaceId, roleId]) => {
			if (!auth || auth.type !== "LoggedIn" || !workspaceId || !roleId) {
				return;
			}
			console.log("Fetching role info for workspaceId:", workspaceId, "roleId:", roleId);
			const response = await httpRequest<GetRoleInfoResponse>(
				`${import.meta.env.VITE_BASE_URL}/api/workspace/${workspaceId}/rbac/role/${roleId}`,
				{
					method: "GET",
				}
			);
			console.log("Role info response:", response.data);
			if (!response.ok) {
				console.error("Failed to fetch role info:", response.data.error);
				toast("Failed to fetch role info", "error");
				return;
			}

			return response.data;
		}
	);

	return (
		<>
			<Title>Role Details | Patr</Title>
			<PageContainer>
				<RoleHeader roleName={roleInfo()?.name} workspaceName={workspaceInfo()?.name} activeTab={activeTab()} />
				<PageContainerBody class="flex flex-col gap-6">
					<ErrorBoundary
						fallback={(err, reset) => (
							<div class="text-white">
								<p>Error loading role information: {err.message}</p>
								<button onClick={reset}>Retry</button>
							</div>
						)}
					>
						<Suspense
							fallback={
								<div class="flex items-center justify-center py-8">
									<div class="text-gray-400">Loading role information...</div>
								</div>
							}
						>
							<Show when={roleInfo()} fallback={null}>
								<div class="flex flex-col gap-4">
									<Show when={activeTab() === "permissions"}>
										<EditPermissions refetchRoleInfo={refetchRoleInfo} roleInfo={roleInfo} />
									</Show>

									<Show when={activeTab() === "users"}>
										<UsersAssignedToRole />
									</Show>
								</div>
							</Show>
						</Suspense>
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
