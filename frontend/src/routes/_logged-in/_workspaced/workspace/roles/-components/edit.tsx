import { Button, ButtonVariant, Table, TableRow, TableCell, useToast } from "~/components";
import PermissionSelector from "./permission-selector";
import { createEffect, createMemo, createSignal, For, Show, Suspense } from "solid-js";
import { useParams } from "@tanstack/solid-router";
import { httpRequest } from "~/utils/http-request";
import { UpdateRoleRequest } from "~/bindings/UpdateRoleRequest";
import { createLoggedInAction } from "~/hooks";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { ResourcePermissionType } from "~/bindings";
import { usePermissionsQuery, useRoleInfoQuery } from "~/hooks/fetch";
import { roleKeys } from "~/hooks/query-keys";
import { useQueryClient } from "@tanstack/solid-query";
import { FiTrash2, FiX } from "solid-icons/fi";
import { parsePermissionName, parseCamelCase } from "~/utils/func";
import { useResourceListQuery } from "~/components/list-resources";

const MAX_AUTO_PAGES = 10;

const ResourcesCell = (props: {
	workspaceId: string;
	resourceType: string;
	resourceIds: string[];
	permissionType: "include" | "exclude";
	onRemove: (resourceId: string) => void;
}) => {
	const resourcesQuery = useResourceListQuery(
		() => props.workspaceId,
		() => props.resourceType
	);

	const nameMap = createMemo(() => {
		const map = new Map<string, string>();
		resourcesQuery.data?.pages.forEach((page) => {
			page.items.forEach((item) => map.set(item.id, item.name));
		});
		return map;
	});

	const unresolvedCount = createMemo(() => {
		const map = nameMap();
		return props.resourceIds.filter((id) => !map.has(id)).length;
	});

	let autoPagesFetched = 0;
	createEffect(() => {
		if (
			unresolvedCount() > 0 &&
			resourcesQuery.hasNextPage &&
			!resourcesQuery.isFetchingNextPage &&
			autoPagesFetched < MAX_AUTO_PAGES
		) {
			autoPagesFetched += 1;
			resourcesQuery.fetchNextPage();
		}
	});

	return (
		<div class="flex flex-wrap items-center gap-1.5">
			<span class="text-sm text-gray-400 mr-1">
				{props.permissionType === "include" ? "Only:" : "All except:"}
			</span>
			<For each={props.resourceIds}>
				{(id) => {
					const resolved = () => nameMap().get(id);
					const label = () => resolved() ?? `${id.slice(0, 8)}…`;
					return (
						<span class={`chip-tag ${resolved() ? "" : "opacity-60"}`}>
							{label()}
							<button
								type="button"
								class="flex items-center justify-center w-4 h-4 rounded-sm bg-white/10 hover:bg-white/20 transition-colors"
								aria-label={`Remove ${label()}`}
								onClick={(e) => {
									e.stopPropagation();
									props.onRemove(id);
								}}
							>
								<FiX size={10} color="#9ca3af" />
							</button>
						</span>
					);
				}}
			</For>
			<Show when={unresolvedCount() > 0 && resourcesQuery.hasNextPage}>
				<button
					type="button"
					class="text-xs text-primary hover:underline disabled:opacity-50"
					disabled={resourcesQuery.isFetchingNextPage}
					onClick={() => resourcesQuery.fetchNextPage()}
				>
					{resourcesQuery.isFetchingNextPage
						? "Loading…"
						: `+ Show ${unresolvedCount()} more`}
				</button>
			</Show>
		</div>
	);
};

const EditPermissions = () => {
	const [permissionsData, setPermissionsData] = createSignal<{ [key: string]: ResourcePermissionType }>({});
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

	const { execute: handleUpdateRole, isLoading: isUpdating } = createLoggedInAction(async () => {
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
		const wsId = workspaceId();
		if (wsId) {
			queryClient.invalidateQueries({ queryKey: roleKeys.detail(wsId, params().roleId) });
		}
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
					column_grids={["flex-4", "flex-3", "flex-4", "flex-1"]}
					headings={["Resource Type", "Action", "Resources", ""]}
					rows={permissionEntries().sort(
						(a, b) => a.resourceType.localeCompare(b.resourceType) || a.action.localeCompare(b.action)
					)}
					renderRow={(perm) => (
						<TableRow>
							<TableCell index={0}>
								<span class="truncate">{parseCamelCase(perm.resourceType)}</span>
							</TableCell>
							<TableCell index={1}>
								<span>{parseCamelCase(perm.action)}</span>
							</TableCell>
							<TableCell index={2}>
								<Show
									when={perm.resources.length > 0}
									fallback={<span class="text-gray-400">All resources</span>}
								>
									<ResourcesCell
										workspaceId={workspaceId()!}
										resourceType={perm.resourceType}
										resourceIds={perm.resources}
										permissionType={perm.permissionType as "include" | "exclude"}
										onRemove={(resourceId) => {
											setPermissionsData((prev) => {
												const current = prev[perm.permissionId];
												if (!current) return prev;
												const nextResources = current.resources.filter((r) => r !== resourceId);
												if (
													current.permissionType === "include" &&
													nextResources.length === 0
												) {
													const next = { ...prev };
													delete next[perm.permissionId];
													return next;
												}
												return {
													...prev,
													[perm.permissionId]: {
														...current,
														resources: nextResources,
													},
												};
											});
										}}
									/>
								</Show>
							</TableCell>
							<TableCell index={3} align="center">
								<button
									type="button"
									aria-label="Remove permission"
									class="text-error hover:bg-white/10 p-1 rounded transition-colors cursor-pointer"
									onClick={() => {
										const newPermissionsData = { ...permissionsData() };
										delete newPermissionsData[perm.permissionId];
										setPermissionsData(newPermissionsData);
									}}
								>
									<FiTrash2 size={16} />
								</button>
							</TableCell>
						</TableRow>
					)}
				/>
			</div>
		</Suspense>
	);
};

export default EditPermissions;
