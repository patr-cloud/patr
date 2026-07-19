import { Alert, Button, ButtonVariant, Input, Table, useToast } from "~/components";
import PermissionSelector from "./permission-selector";
import { createEffect, createMemo, createSignal, Show, Suspense } from "solid-js";
import { useParams } from "@tanstack/solid-router";
import { httpRequest } from "~/utils/http-request";
import { UpdateRoleRequest } from "~/bindings/UpdateRoleRequest";
import { createLoggedInAction } from "~/hooks";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { ResourcePermissionType } from "~/bindings";
import { usePermissionsQuery, useResourcesInfoQuery, useRoleInfoQuery } from "~/hooks/fetch";
import { roleKeys } from "~/hooks/query-keys";
import { useQueryClient } from "@tanstack/solid-query";
import PermissionRow, { removeResourceFromPermissions } from "./permission-row";
import { parsePermissionName } from "~/utils/func";
import { validateNameField, validateRoleDescription } from "~/utils/validation";

const EditPermissions = () => {
	const [permissionsData, setPermissionsData] = createSignal<{ [key: string]: ResourcePermissionType }>({});
	const [roleName, setRoleName] = createSignal("");
	const [roleDescription, setRoleDescription] = createSignal("");
	const [roleNameError, setRoleNameError] = createSignal<string | undefined>(undefined);
	const [roleDescriptionError, setRoleDescriptionError] = createSignal<string | undefined>(undefined);
	const [workspaceId] = useLastWorkspaceId();
	const toast = useToast();
	const queryClient = useQueryClient();
	const params = useParams({ from: "/_logged-in/_workspaced/workspace/roles/$roleId" });

	const roleInfoQuery = useRoleInfoQuery(() => params().roleId);

	// Initialize permissions data when role info loads
	createEffect(() => {
		const role = roleInfoQuery.data;
		if (role) {
			setPermissionsData(role.permissions as { [key: string]: ResourcePermissionType });
			setRoleName(role.name);
			setRoleDescription(role.description ?? "");
		}
	});

	// Fetch all permissions for the workspace to map IDs to names
	const allPermissionsQuery = usePermissionsQuery(() => workspaceId()!);

	// Create a map of permission ID to permission name
	const permissionIdToName = createMemo(() => {
		const perms = allPermissionsQuery.data?.permissions;
		if (!perms) return new Map<string, string>();
		return new Map(perms.map((perm) => [perm.id, perm.name]));
	});

	const permissionEntries = createMemo(() => {
		const permissions = permissionsData();
		if (!permissions) return [];
		const nameMap = permissionIdToName();

		return Object.entries(permissions).map(([permissionId, permissionData]) => {
			const permissionName = nameMap.get(permissionId) || permissionId;
			const parsed = parsePermissionName(permissionName);
			return {
				permissionId,
				resourceType: parsed.resourceType,
				action: parsed.permission,
				permissionType: permissionData?.permissionType || "exclude",
				resources: permissionData?.resources || [],
			};
		});
	});

	// Every resource referenced by the table, resolved in a single request so the
	// rows can show what the stored IDs actually refer to.
	const allResourceIds = createMemo(() => [...new Set(permissionEntries().flatMap((perm) => perm.resources))]);
	const resourcesInfoQuery = useResourcesInfoQuery(() => allResourceIds());

	const { execute: handleUpdateRole, isLoading: isUpdating } = createLoggedInAction(async () => {
		const nameError = validateNameField(roleName());
		const descError = validateRoleDescription(roleDescription());
		setRoleNameError(nameError);
		setRoleDescriptionError(descError);
		if (nameError || descError) return;

		const requestBody: UpdateRoleRequest = {
			name: roleName().trim(),
			description: roleDescription().trim(),
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
		const wsId = workspaceId();
		if (wsId) {
			queryClient.invalidateQueries({ queryKey: roleKeys.detail(wsId, params().roleId) });
		}
	});

	return (
		<Suspense fallback={<div class="text-gray-400 text-center py-8">Loading role information...</div>}>
			<div class="flex flex-col gap-4">
				<div class="flex justify-between items-center">
					<h3 class="text-lg text-white">Edit Role</h3>

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

				<div class="flex flex-col gap-2">
					<label class="text-white text-sm">Role Name</label>
					<Input
						type="text"
						placeholder="Enter Name"
						value={roleName()}
						onInput={(e) => {
							setRoleName(e.currentTarget.value);
							setRoleNameError(undefined);
						}}
					/>
					<Show when={roleNameError()}>
						<Alert message={roleNameError()!} type="error" />
					</Show>
				</div>

				<div class="flex flex-col gap-2">
					<label class="text-white text-sm">Description</label>
					<Input
						type="text"
						placeholder="Enter Description (optional)"
						value={roleDescription()}
						onInput={(e) => {
							setRoleDescription(e.currentTarget.value);
							setRoleDescriptionError(undefined);
						}}
					/>
					<Show when={roleDescriptionError()}>
						<Alert message={roleDescriptionError()!} type="error" />
					</Show>
				</div>

				<div class="flex items-center gap-2">
					<PermissionSelector
						class="flex-1"
						workspaceId={workspaceId()!}
						permissionsData={permissionsData()}
						onPermissionsDataChange={(data) => setPermissionsData((prev) => ({ ...prev, ...data }))}
					/>
				</div>

				<Table
					column_grids={["flex-4", "flex-3", "flex-4", "flex-1"]}
					headings={["Resource Type", "Action", "Resources", ""]}
					heading_align="left"
					rows={permissionEntries().sort(
						(a, b) => a.resourceType.localeCompare(b.resourceType) || a.action.localeCompare(b.action)
					)}
					renderRow={(perm) => (
						<PermissionRow
							perm={perm}
							resourceInfo={resourcesInfoQuery.data}
							isLoadingResources={resourcesInfoQuery.isPending}
							onRemove={() => {
								const newPermissionsData = { ...permissionsData() };
								delete newPermissionsData[perm.permissionId];
								setPermissionsData(newPermissionsData);
							}}
							onRemoveResource={(resourceId) =>
								setPermissionsData((prev) =>
									removeResourceFromPermissions(prev, perm.permissionId, resourceId)
								)
							}
						/>
					)}
				/>
			</div>
		</Suspense>
	);
};

export default EditPermissions;
