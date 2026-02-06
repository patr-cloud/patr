import { createEffect, createMemo, createResource, createSignal, ErrorBoundary, For, Show, Suspense } from "solid-js";
import { useNavigate, useParams, useSearchParams } from "@solidjs/router";
import { Button, ButtonVariant, PageContainer, PageContainerBody, Table, useToast } from "~/components";
import { useAuthState } from "~/hooks";
import { GetRoleInfoResponse } from "~/bindings/GetRoleInfoResponse";
import { GetWorkspaceInfoResponse } from "~/bindings/GetWorkspaceInfoResponse";
import { ListUsersForRoleResponse } from "~/bindings/ListUsersForRoleResponse";
import { GetUserDetailsResponse } from "~/bindings/GetUserDetailsResponse";
import { UpdateRoleRequest } from "~/bindings/UpdateRoleRequest";
import { ResourcePermissionType } from "~/bindings/ResourcePermissionType";
import { ListAllPermissionsResponse } from "~/bindings/ListAllPermissionsResponse";
import { httpRequest } from "~/utils/http-request";
import RoleHeader from "./role-header";
import PermissionSelector from "./permission-selector";

const RoleInfo = () => {
	const params = useParams();
	const [searchParams] = useSearchParams();
	const [authState] = useAuthState();
	const toast = useToast();
	const navigate = useNavigate();

	const activeTab = createMemo(() => (searchParams.tab === "users" ? "users" : "permissions"));
	const [isEditing, setIsEditing] = createSignal(false);
	const [selectedPermissionIds, setSelectedPermissionIds] = createSignal<Set<string>>(new Set());
	const [permissionsData, setPermissionsData] = createSignal<{ [key: string]: ResourcePermissionType }>({});
	const [isUpdating, setIsUpdating] = createSignal(false);

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

	// Fetch all permissions for the workspace to map IDs to names
	const [allPermissions] = createResource(
		() => [authState(), params.id] as const,
		async ([auth, workspaceId]) => {
			if (!auth || auth.type !== "LoggedIn" || !workspaceId) {
				return;
			}

			const response = await httpRequest<ListAllPermissionsResponse>(
				`${import.meta.env.VITE_BASE_URL}/api/workspace/${workspaceId}/rbac/permission`,
				{
					method: "GET",
					headers: {
						"Content-Type": "application/json",
						Authorization: `Bearer ${auth.accessToken}`,
					},
				}
			);

			if (!response.ok) {
				console.error("Failed to fetch permissions:", response.data.error);
				return;
			}

			return response.data;
		}
	);

	const [usersData] = createResource(
		() => [authState(), params.id, params.roleId, activeTab()] as const,
		async ([auth, workspaceId, roleId, tab]) => {
			if (!auth || auth.type !== "LoggedIn" || !workspaceId || !roleId || tab !== "users") {
				return;
			}

			const response = await httpRequest<ListUsersForRoleResponse>(
				`${import.meta.env.VITE_BASE_URL}/api/workspace/${workspaceId}/rbac/role/${roleId}/users`,
				{
					method: "GET",
					headers: {
						"Content-Type": "application/json",
						Authorization: `Bearer ${auth.accessToken}`,
					},
				}
			);

			console.log("Users for role response:", response.data);
			if (!response.ok) {
				console.error("Failed to fetch users for role:", response.data.error);
				toast("Failed to fetch users for role", "error");
				return;
			}

			return response.data;
		}
	);

	// Create a map of permission ID to permission name
	const permissionIdToName = createMemo(() => {
		const perms = allPermissions()?.permissions;
		if (!perms) return new Map<string, string>();
		return new Map(perms.map((perm) => [perm.id, perm.name]));
	});

	const permissionEntries = createMemo(() => {
		const permissions = roleInfo()?.permissions;
		if (!permissions) return [];
		const nameMap = permissionIdToName();
		return Object.entries(permissions).map(([permissionId, permissionData]) => ({
			permissionId,
			permissionName: nameMap.get(permissionId) || permissionId,
			permissionType: permissionData?.permissionType || "all",
			resources: permissionData?.permissionType ? permissionData.resources : undefined,
		}));
	});

	const usersList = createMemo(() => {
		const users = usersData()?.users;
		if (!users) return [];
		return users.map((userId) => ({ userId }));
	});

	// Fetch user details for each user ID
	const [usersDetails] = createResource(
		() => [authState(), usersList(), activeTab()] as const,
		async ([auth, users, tab]) => {
			if (!auth || auth.type !== "LoggedIn" || users.length === 0 || tab !== "users") {
				return [];
			}

			// Fetch all user details in parallel
			const userDetailsPromises = users.map(async (user) => {
				const response = await httpRequest<GetUserDetailsResponse>(
					`${import.meta.env.VITE_BASE_URL}/api/user/${user.userId}`,
					{
						method: "GET",
						headers: {
							"Content-Type": "application/json",
							Authorization: `Bearer ${auth.accessToken}`,
						},
					}
				);

				if (response.ok) {
					return response.data;
				}
				return null;
			});

			const results = await Promise.all(userDetailsPromises);
			return results.filter((user) => user !== null) as GetUserDetailsResponse[];
		}
	);

	const usersWithDetails = createMemo(() => {
		return usersDetails() || [];
	});

	// Initialize selected permissions when role info loads and editing starts
	createEffect(() => {
		const role = roleInfo();
		if (role && isEditing()) {
			const permIds = Object.keys(role.permissions);
			setSelectedPermissionIds(new Set(permIds));
			setPermissionsData(role.permissions as { [key: string]: ResourcePermissionType });
		}
	});

	const handleUpdateRole = async () => {
		const auth = authState();
		if (!auth || auth.type !== "LoggedIn") {
			toast("You must be logged in to update a role", "error");
			return;
		}

		if (selectedPermissionIds().size === 0) {
			toast("Please select at least one permission", "error");
			return;
		}

		setIsUpdating(true);

		try {
			const requestBody: UpdateRoleRequest = {
				permissions: permissionsData(),
			};

			const response = await httpRequest(
				`${import.meta.env.VITE_BASE_URL}/api/workspace/${params.id}/rbac/role/${params.roleId}`,
				{
					method: "PATCH",
					headers: {
						"Content-Type": "application/json",
						Authorization: `Bearer ${auth.accessToken}`,
					},
					body: JSON.stringify(requestBody),
				}
			);

			if (!response.ok) {
				console.error("Failed to update role:", response.data.error);
				toast(response.data.error || "Failed to update role", "error");
				return;
			}

			toast("Role updated successfully", "success");
			setIsEditing(false);
			// Navigate back to roles list
			navigate(`/workspaces/${params.id}/roles`);
		} catch (error) {
			console.error("Error updating role:", error);
			toast("An error occurred while updating the role", "error");
		} finally {
			setIsUpdating(false);
		}
	};

	return (
		<PageContainer>
			<RoleHeader
				roleName={roleInfo()?.role.name}
				workspaceName={workspaceInfo()?.name}
				activeTab={activeTab()}
			/>
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
									<div class="flex flex-col gap-4">
										<div class="flex justify-between items-center">
											<h3 class="text-lg text-white">Permissions</h3>
											<Button variant={ButtonVariant.Outlined} onClick={() => setIsEditing(true)}>
												Edit Permissions
											</Button>
										</div>
										<Show when={isEditing()}>
											<div class="flex flex-col gap-4">
												<PermissionSelector
													workspaceId={params.id!}
													selectedPermissionIds={selectedPermissionIds()}
													onPermissionChange={setSelectedPermissionIds}
													onPermissionsDataChange={setPermissionsData}
												/>
												<div class="flex justify-end gap-4">
													<Button
														variant={ButtonVariant.Outlined}
														onClick={() => {
															setIsEditing(false);
															setSelectedPermissionIds(new Set<string>());
														}}
														disabled={isUpdating()}
													>
														Cancel
													</Button>
													<Button
														variant={ButtonVariant.Contained}
														onClick={handleUpdateRole}
														disabled={isUpdating() || selectedPermissionIds().size === 0}
													>
														{isUpdating() ? "Updating..." : "Save Changes"}
													</Button>
												</div>
											</div>
										</Show>

										<Show when={!isEditing()}>
											<Show
												when={permissionEntries().length > 0}
												fallback={<div class="text-gray-400">No permissions assigned to this role</div>}
											>
												<Table
													column_grids={["flex-3", "flex-2", "flex-2"]}
													headings={["Permission Name", "Permission Type", "Resources"]}
													rows={permissionEntries()}
													renderRow={(item) => (
														<tr class="table-row">
															<td class="flex-3 flex items-center justify-center">
																<span class="truncate">{item.permissionName}</span>
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
										</Show>
									</div>

								</Show>

								<Show when={activeTab() === "users"}>
									<div class="flex flex-col gap-4">
										<div class="flex items-center justify-between">
											<h3 class="text-lg text-white">Users with this role</h3>
											<span class="text-gray-400 text-sm">{usersWithDetails().length} users</span>
										</div>
										<Show
											when={usersWithDetails().length > 0}
											fallback={
												<div class="text-gray-400 text-center py-8">
													No users have been assigned this role yet
												</div>
											}
										>
											<Table
												column_grids={["flex-1"]}
												headings={["Username"]}
												rows={usersWithDetails()}
												renderRow={(item) => (
													<tr class="table-row">
														<td class="flex-2 flex items-center justify-center">
															<span class="truncate font-mono">{item.username}</span>
														</td>
													</tr>
												)}
											/>
										</Show>
									</div>
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
