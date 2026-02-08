import { Button, ButtonVariant, Table, useToast } from "~/components";
import PermissionSelector from "../permission-selector";
import { createEffect, createMemo, createSignal, For, Resource, Show, Suspense } from "solid-js";
import { useNavigate, useParams } from "@solidjs/router";
import { httpRequest } from "~/utils/http-request";
import { UpdateRoleRequest } from "~/bindings/UpdateRoleRequest";
import { useAuthState } from "~/hooks";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { GetRoleInfoResponse, ResourcePermissionType } from "~/bindings";
import useFetchPermissions from "~/hooks/use-fetch/use-fetch-permissions";

const EditPermissions = (props: { roleInfo: Resource<GetRoleInfoResponse | undefined> }) => {
	const [selectedPermissionIds, setSelectedPermissionIds] = createSignal<Set<string>>(new Set());
	const [isUpdating, setIsUpdating] = createSignal(false);
	const [permissionsData, setPermissionsData] = createSignal<{ [key: string]: ResourcePermissionType }>({});
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();
	const toast = useToast();
	const navigate = useNavigate();
	const params = useParams();

	// Initialize selected permissions when role info loads and editing starts
	createEffect(() => {
		const role = props.roleInfo();
		if (role) {
			const permIds = Object.keys(role.permissions);
			setSelectedPermissionIds(new Set(permIds));
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
		return Object.entries(permissions).map(([permissionId, permissionData]) => ({
			permissionId,
			permissionName: nameMap.get(permissionId) || permissionId,
			permissionType: permissionData?.permissionType || "all",
			resources: permissionData?.permissionType ? permissionData.resources : undefined,
		}));
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
				`${import.meta.env.VITE_BASE_URL}/api/workspace/${workspaceId()}/rbac/role/${params.roleId}`,
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
			// Navigate back to roles list
			navigate("/workspace/roles");
		} catch (error) {
			console.error("Error updating role:", error);
			toast("An error occurred while updating the role", "error");
		} finally {
			setIsUpdating(false);
		}
	};

	return (
		<Suspense fallback={<div class="text-gray-400 text-center py-8">Loading role information...</div>}>
			<div class="flex flex-col gap-4">
				<div class="flex justify-between items-center">
					<h3 class="text-lg text-white">Add Permission</h3>
				</div>

				<div class="flex flex-col gap-4">
					<PermissionSelector
						workspaceId={workspaceId()!}
						selectedPermissionIds={selectedPermissionIds()}
						onPermissionChange={(ids) => setSelectedPermissionIds((prev) => new Set([...prev, ...ids]))}
						onPermissionsDataChange={setPermissionsData}
					/>
					<div class="flex justify-end gap-4">
						<button
							onClick={(e) => {
								e.preventDefault();

								console.log(permissionsData());
								console.log(permissionIdToName());
								console.log(permissionEntries());
							}}
						>
							console log
						</button>
						<Button
							variant={ButtonVariant.Contained}
							onClick={handleUpdateRole}
							disabled={isUpdating() || selectedPermissionIds().size === 0}
						>
							{isUpdating() ? "Updating..." : "Save Changes"}
						</Button>
					</div>
				</div>

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
										<For each={item.resources}>{(resource) => <span class="text-sm">{resource}</span>}</For>
									</div>
								</Show>
							</td>
						</tr>
					)}
				/>
			</div>
		</Suspense>
	);
};

export default EditPermissions;
