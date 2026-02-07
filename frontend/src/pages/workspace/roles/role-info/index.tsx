import { createMemo, createResource, ErrorBoundary, Show, Suspense } from "solid-js";
import { useParams, useSearchParams } from "@solidjs/router";
import { PageContainer, PageContainerBody, useToast } from "~/components";
import { useAuthState } from "~/hooks";
import { GetRoleInfoResponse } from "~/bindings/GetRoleInfoResponse";
import { GetWorkspaceInfoResponse } from "~/bindings/GetWorkspaceInfoResponse";
import { httpRequest } from "~/utils/http-request";
import RoleHeader from "~/pages/workspace/roles/role-header";
import UsersAssignedToRole from "./users";
import EditPermissions from "./edit";

const RoleInfo = () => {
	const params = useParams();
	const [searchParams] = useSearchParams();
	const [authState] = useAuthState();
	const toast = useToast();

	const activeTab = createMemo(() => (searchParams.tab === "users" ? "users" : "permissions"));

	const fetchParams = createMemo(() => {
		return [authState(), params.id, params.roleId] as const;
	});

	const [workspaceInfo] = createResource(
		() => [authState(), params.id] as const,
		async ([auth, workspaceId]) => {
			if (!auth || auth.type !== "LoggedIn" || !workspaceId) {
				return;
			}

			const response = await httpRequest<GetWorkspaceInfoResponse>(
				`${import.meta.env.VITE_BASE_URL}/api/workspace/${workspaceId}`,
				{
					method: "GET",
					headers: {
						"Content-Type": "application/json",
						Authorization: `Bearer ${auth.accessToken}`,
					},
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

	const [roleInfo] = createResource(fetchParams, async ([auth, workspaceId, roleId]) => {
		if (!auth || auth.type !== "LoggedIn" || !workspaceId || !roleId) {
			return;
		}
		console.log("Fetching role info for workspaceId:", workspaceId, "roleId:", roleId);
		const response = await httpRequest<GetRoleInfoResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${workspaceId}/rbac/role/${roleId}`,
			{
				method: "GET",
				headers: {
					"Content-Type": "application/json",
					Authorization: `Bearer ${auth.accessToken}`,
				},
			}
		);
		console.log("Role info response:", response.data);
		if (!response.ok) {
			console.error("Failed to fetch role info:", response.data.error);
			toast("Failed to fetch role info", "error");
			return;
		}

		return response.data;
	});

	return (
		<PageContainer>
			<RoleHeader roleName={roleInfo()?.role.name} workspaceName={workspaceInfo()?.name} activeTab={activeTab()} />
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
									<EditPermissions roleInfo={roleInfo} />
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
	);
};

export default RoleInfo;
