import { Button, ButtonVariant, Table, useToast } from "~/components";
import PermissionSelector from "./permission-selector";
import { createEffect, createMemo, createSignal, For, Resource, Show, Suspense } from "solid-js";
import { useParams } from "@tanstack/solid-router";
import { httpRequest } from "~/utils/http-request";
import { UpdateRoleRequest } from "~/bindings/UpdateRoleRequest";
import { createLoggedInAction } from "~/hooks";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { GetRoleInfoResponse, ResourcePermissionType } from "~/bindings";
import { useFetchPermissions } from "~/hooks/fetch";
import { FiTrash2, FiXCircle } from "solid-icons/fi";
import { parsePermissionName } from "~/utils/func";

const EditPermissions = (props: {
	roleInfo: Resource<GetRoleInfoResponse | undefined>;
	refetchRoleInfo: () => void;
}) => {
	const [selectedPermissionIds, setSelectedPermissionIds] = createSignal<Set<string>>(new Set());
	const [permissionsData, setPermissionsData] = createSignal<{ [key: string]: ResourcePermissionType }>({});
	const [workspaceId] = useLastWorkspaceId();
	const toast = useToast();
	const params = useParams({ from: "/_logged-in/_workspaced/workspace/roles/$roleId" });

	// Initialize selected permissions when role info loads and editing starts
	createEffect(() => {
		const role = props.roleInfo();
		if (role) {
			const permIds = Object.keys(role.permissions);
			// setSelectedPermissionIds(new Set(permIds));
			setPermissionsData(role.permissions as { [key: string]: ResourcePermissionType });
		}
	});

	// Fetch all permissions for the workspace to map IDs to names
	const [allPermissions] = useFetchPermissions(workspaceId()!);

	// Create a map of permission ID to permission name
	const permissionIdToName = createMemo(() => {
		const perms = allPermissions()?.permissions;
		if (!perms) return new Map<string, string>();
		return new Map(perms.map((perm) => [perm.id, perm.name]));
	});

	const permissionEntries = createMemo(() => {
		const permissions = permissionsData();
		if (!permissions) return [];
		const nameMap = permissionIdToName();

		// Group permissions by resourceType
		const grouped = new Map<
			string,
			{
				permissionResourceType: string;
				permissionActions: Array<{ permissionId: string; action: string }>;
				permissionType: string;
				resources?: string[];
			}
		>();

		Object.entries(permissions).forEach(([permissionId, permissionData]) => {
			const permissionName = nameMap.get(permissionId) || permissionId;
			const parsed = parsePermissionName(permissionName);

			if (!grouped.has(parsed.resourceType)) {
				grouped.set(parsed.resourceType, {
					permissionResourceType: parsed.resourceType,
					permissionActions: [],
					permissionType: permissionData?.permissionType || "all",
					resources: permissionData?.permissionType ? permissionData.resources : undefined,
				});
			}

			const group = grouped.get(parsed.resourceType)!;
			group.permissionActions.push({
				permissionId,
				action: parsed.action,
			});
		});

		return Array.from(grouped.values());
	});

	const { execute: handleUpdateRole, isLoading: isUpdating } = createLoggedInAction(async ({ accessToken }) => {
		const requestBody: UpdateRoleRequest = {
			permissions: permissionsData(),
		};

		const response = await httpRequest(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${workspaceId()}/rbac/role/${params().roleId}`,
			{
				method: "PATCH",
				body: JSON.stringify(requestBody),
			}
		);

		if (!response.ok) {
			console.error("Failed to update role:", response.data.error);
			toast(response.data.error || "Failed to update role", "error");
			return;
		}

		toast("Role updated successfully", "success");
		// Navigate back to roles list
		props.refetchRoleInfo();
	});

	return (
		<Suspense fallback={<div class="text-gray-400 text-center py-8">Loading role information...</div>}>
			<div class="flex flex-col gap-4">
				<div class="flex justify-between items-center">
					<h3 class="text-lg text-white">Edit Permissions</h3>

					<div class="flex justify-end gap-4">
						<Button
							variant={ButtonVariant.Contained}
							onClick={() => handleUpdateRole().catch(() => {})}
							disabled={isUpdating() || Object.keys(permissionsData()).length === 0}
						>
							{isUpdating() ? "Saving Changes..." : "Save Changes"}
						</Button>
					</div>
				</div>

				<div class="flex items-center gap-2">
					<PermissionSelector
						class="flex-1"
						workspaceId={workspaceId()!}
						selectedPermissionIds={selectedPermissionIds()}
						onPermissionChange={setSelectedPermissionIds}
						onPermissionsDataChange={(data) => setPermissionsData((prev) => ({ ...prev, ...data }))}
					/>
				</div>

				<Table
					column_grids={["flex-2", "flex-3", "flex-2"]}
					headings={["Resource Type", "Actions", "Resources"]}
					rows={permissionEntries().sort((a, b) =>
						a.permissionResourceType.localeCompare(b.permissionResourceType)
					)}
					renderRow={(perm) => (
						<tr class="table-row">
							<td class="flex-2 flex items-center justify-center">
								<span class="truncate">{perm.permissionResourceType}</span>
							</td>
							<td class="flex-3 flex items-center justify-center">
								<div class="flex flex-wrap gap-1 justify-center">
									<For each={perm.permissionActions}>
										{(actionData) => (
											<span
												onClick={() => {
													const newPermissionsData = { ...permissionsData() };
													delete newPermissionsData[actionData.permissionId];
													setPermissionsData(newPermissionsData);
												}}
												class="text-sm px-2 py-1 bg-secondary-medium rounded cursor-pointer hover:bg-secondary-dark transition-colors flex items-center justify-center gap-1"
											>
												{actionData.action}
												<FiXCircle size={12} class="inline-block" />
											</span>
										)}
									</For>
								</div>
							</td>
							<td class="flex-2 flex items-center justify-center">
								<Show
									when={perm.resources && perm.resources.length > 0}
									fallback={<span class="text-gray-400">All resources</span>}
								>
									<div class="flex flex-col gap-1">
										<For each={perm.resources}>
											{(resource) => <span class="text-sm">{resource}</span>}
										</For>
									</div>
								</Show>
							</td>
							<td
								onClick={() => {
									const newPermissionsData = { ...permissionsData() };
									// Delete all permissions for this resource type
									perm.permissionActions.forEach((actionData) => {
										delete newPermissionsData[actionData.permissionId];
									});
									setPermissionsData(newPermissionsData);
								}}
							>
								<FiTrash2 color="red" />
							</td>
						</tr>
					)}
				/>
			</div>
		</Suspense>
	);
};

export default EditPermissions;
