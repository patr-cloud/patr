import { Button, ButtonVariant, Table, useToast } from "~/components";
import PermissionSelector from "./permission-selector";
import { createEffect, createMemo, createSignal, Resource, Show, Suspense } from "solid-js";
import { useParams } from "@tanstack/solid-router";
import { httpRequest } from "~/utils/http-request";
import { UpdateRoleRequest } from "~/bindings/UpdateRoleRequest";
import { createLoggedInAction } from "~/hooks";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { GetRoleInfoResponse, ResourcePermissionType } from "~/bindings";
import { useFetchPermissions } from "~/hooks/fetch";
import { FiTrash2 } from "solid-icons/fi";
import { parsePermissionName, parseCamelCase } from "~/utils/func";

const EditPermissions = (props: {
	roleInfo: Resource<GetRoleInfoResponse | undefined>;
	refetchRoleInfo: () => void;
}) => {
	const [permissionsData, setPermissionsData] = createSignal<{ [key: string]: ResourcePermissionType }>({});
	const [workspaceId] = useLastWorkspaceId();
	const toast = useToast();
	const params = useParams({ from: "/_logged-in/_workspaced/workspace/roles/$roleId" });

	// Initialize permissions data when role info loads
	createEffect(() => {
		const role = props.roleInfo();
		if (role) {
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
						onPermissionsDataChange={(data) => setPermissionsData((prev) => ({ ...prev, ...data }))}
					/>
				</div>

				<Table
					column_grids={["flex-3", "flex-2", "flex-3", "flex-[0.5]"]}
					headings={["Resource Type", "Action", "Resources", ""]}
					rows={permissionEntries().sort(
						(a, b) => a.resourceType.localeCompare(b.resourceType) || a.action.localeCompare(b.action)
					)}
					renderRow={(perm) => (
						<tr class="table-row">
							<td class="flex-3 flex items-center justify-center">
								<span class="truncate">{parseCamelCase(perm.resourceType)}</span>
							</td>
							<td class="flex-2 flex items-center justify-center">
								<span>{parseCamelCase(perm.action)}</span>
							</td>
							<td class="flex-3 flex items-center justify-center">
								<Show
									when={perm.resources.length > 0}
									fallback={<span class="text-gray-400">All resources</span>}
								>
									<span class="text-sm">
										{perm.permissionType === "include" ? "Only " : "All except "}
										{perm.resources.length} resource{perm.resources.length !== 1 ? "s" : ""}
									</span>
								</Show>
							</td>
							<td
								class="flex-[0.5] cursor-pointer"
								onClick={() => {
									const newPermissionsData = { ...permissionsData() };
									delete newPermissionsData[perm.permissionId];
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
