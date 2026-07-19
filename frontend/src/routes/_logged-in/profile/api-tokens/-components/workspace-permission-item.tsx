import { createEffect, createMemo, createSignal, Show } from "solid-js";
import { Checkbox, Radio, Table } from "~/components";
import { ResourcePermissionType, WithId, Workspace, WorkspacePermission } from "~/bindings";
import PermissionSelector from "~/routes/_logged-in/_workspaced/workspace/roles/-components/permission-selector";
import { usePermissionsQuery } from "~/hooks/fetch";
import { parsePermissionName, parseCamelCase } from "~/utils/func";
import { FiTrash2 } from "solid-icons/fi";

interface WorkspacePermissionItemProps {
	workspace: WithId<Workspace>;
	isSuperAdmin: boolean;
	enabled: boolean;
	initialPermission?: WorkspacePermission;
	onToggle: (workspaceId: string, enabled: boolean) => void;
	onPermissionChange: (workspaceId: string, permission: WorkspacePermission) => void;
}

const WorkspacePermissionItem = (props: WorkspacePermissionItemProps) => {
	const [superAdminMode, setSuperAdminMode] = createSignal(props.initialPermission?.type === "superAdmin");
	const extractMemberPermissions = (wp?: WorkspacePermission): { [key: string]: ResourcePermissionType } => {
		if (!wp || wp.type !== "member") return {};
		const { type: _, ...rest } = wp as Record<string, ResourcePermissionType | string>;
		return rest as { [key: string]: ResourcePermissionType };
	};
	const [permissionsData, setPermissionsData] = createSignal<{ [key: string]: ResourcePermissionType }>(
		extractMemberPermissions(props.initialPermission)
	);

	// Fetch all permissions for the workspace to map IDs to names
	const allPermissionsQuery = usePermissionsQuery(() => props.workspace.id);

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
				permission: parsed.permission,
				permissionType: permissionData?.permissionType || "exclude",
				resources: permissionData?.resources || [],
			};
		});
	});

	// Propagate permission changes to parent
	createEffect(() => {
		if (!props.enabled) return;
		if (superAdminMode()) {
			props.onPermissionChange(props.workspace.id, { type: "superAdmin" });
		} else {
			const data = permissionsData();
			props.onPermissionChange(props.workspace.id, { type: "member", ...data } as WorkspacePermission);
		}
	});

	return (
		<div class="w-full flex flex-col items-start gap-2 border border-border-color rounded-xs p-4">
			<Checkbox
				checked={props.enabled}
				onChange={() => props.onToggle(props.workspace.id, !props.enabled)}
				label={props.workspace.name}
			/>

			<Show when={props.enabled}>
				<div class="flex flex-col gap-4 w-full">
					{/* Super Admin toggle (only shown if current user is super admin of this workspace) */}
					<Show when={props.isSuperAdmin}>
						<div class="flex flex-row items-center gap-6 mt-2">
							<Radio
								name={`perm-mode-${props.workspace.id}`}
								checked={superAdminMode()}
								onChange={() => setSuperAdminMode(true)}
								label="Super Admin"
							/>
							<Radio
								name={`perm-mode-${props.workspace.id}`}
								checked={!superAdminMode()}
								onChange={() => setSuperAdminMode(false)}
								label="Custom Permissions"
							/>
						</div>
					</Show>

					{/* Permission Selector (shown when not super admin mode) */}
					<Show when={!superAdminMode()}>
						<PermissionSelector
							class="flex-1 w-full"
							workspaceId={props.workspace.id}
							permissionsData={permissionsData()}
							onPermissionsDataChange={(data) => setPermissionsData((prev) => ({ ...prev, ...data }))}
						/>

						<Show when={permissionEntries().length > 0}>
							<Table
								column_grids={["flex-4", "flex-3", "flex-4", "flex-1"]}
								headings={["Resource Type", "Permission", "Resources", ""]}
								rows={permissionEntries().sort(
									(a, b) =>
										a.resourceType.localeCompare(b.resourceType) ||
										a.permission.localeCompare(b.permission)
								)}
								renderRow={(perm) => (
									<tr class="table-row">
										<td class="flex-4 flex items-center justify-center">
											<span class="truncate">{parseCamelCase(perm.resourceType)}</span>
										</td>
										<td class="flex-3 flex items-center justify-center">
											<span>{parseCamelCase(perm.permission)}</span>
										</td>
										<td class="flex-4 flex items-center justify-center">
											<Show
												when={perm.resources.length > 0}
												fallback={<span class="text-gray-400">All resources</span>}
											>
												<span class="text-sm">
													{perm.permissionType === "include" ? "Only " : "All except "}
													{perm.resources.length} resource
													{perm.resources.length !== 1 ? "s" : ""}
												</span>
											</Show>
										</td>
										<td
											class="flex-1 cursor-pointer"
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
						</Show>
					</Show>
				</div>
			</Show>
		</div>
	);
};

export default WorkspacePermissionItem;
