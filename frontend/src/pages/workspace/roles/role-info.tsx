import { createMemo, createResource, ErrorBoundary, For, Show, Suspense } from "solid-js";
import { useParams } from "@solidjs/router";
import { PageContainer, PageContainerBody, Table, useToast } from "~/components";
import { useAuthState } from "~/hooks";
import { GetWorkspaceInfoResponse, GetRoleInfoResponse } from "~/bindings";
import { httpRequest } from "~/utils/http-request";
// import WorkspaceHeader from "~/pages/workspace/roles/workspace-header";

const RoleInfo = () => {
	const params = useParams();
	const [authState] = useAuthState();
	const toast = useToast();

	const fetchParams = createMemo(() => {
		return [authState(), params.workspaceId, params.roleId] as const;
	});

	const [workspaceInfo] = createResource(
		() => [authState(), params.workspaceId] as const,
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

		if (!response.ok) {
			console.error("Failed to fetch role info:", response.data.error);
			toast("Failed to fetch role info", "error");
			return;
		}

		return response.data;
	});

	const permissionEntries = createMemo(() => {
		const permissions = roleInfo()?.permissions;
		if (!permissions) return [];
		return Object.entries(permissions).map(([permissionId, permissionData]) => ({
			permissionId,
			...permissionData,
		}));
	});

	return (
		<PageContainer>
			{/* <WorkspaceHeader workspaceName={workspaceInfo()?.name} activeTab="roles" /> */}
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
						<Show when={roleInfo()}>
							<div class="flex flex-col gap-4">
								{/* <div class="bg-secondary-light rounded-xs p-lg">
									<h2 class="text-xl text-white mb-2">{roleInfo()?.name}</h2>
									<p class="text-gray-400">{roleInfo()?.description || "No description provided"}</p>
								</div> */}

								<div class="flex flex-col gap-2">
									<h3 class="text-lg text-white">Permissions</h3>
									<Show
										when={permissionEntries().length > 0}
										fallback={<div class="text-gray-400">No permissions assigned to this role</div>}
									>
										<Table
											column_grids={["flex-3", "flex-2", "flex-2"]}
											headings={["Permission ID", "Permission Type", "Resources"]}
											rows={permissionEntries()}
											renderRow={(item) => (
												<tr class="table-row">
													<td class="flex-3 flex items-center justify-center">
														<span class="truncate">{item.permissionId}</span>
													</td>
													<td class="flex-2 flex items-center justify-center">{item.permissionType}</td>
													<td class="flex-2 flex items-center justify-center">
														<Show
															when={item.resources && item.resources.length > 0}
															fallback={<span class="text-gray-400">All resources</span>}
														>
															<div class="flex flex-col gap-1">
																<For each={item.resources}>
																	{(resource) => <span class="text-sm">{resource}</span>}
																</For>
															</div>
														</Show>
													</td>
												</tr>
											)}
										/>
									</Show>
								</div>
							</div>
						</Show>
					</Suspense>
				</ErrorBoundary>
			</PageContainerBody>
		</PageContainer>
	);
};

export default RoleInfo;
