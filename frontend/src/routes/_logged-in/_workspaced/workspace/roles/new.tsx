import { createFileRoute } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import { createMemo, createResource, createSignal, Show } from "solid-js";
import { useNavigate } from "@tanstack/solid-router";
import { Button, ButtonVariant, Input, PageContainer, PageContainerBody, Table, useToast } from "~/components";
import { createAuthenticatedAction, useAuthState } from "~/hooks";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { GetWorkspaceInfoResponse } from "~/bindings/GetWorkspaceInfoResponse";
import { CreateNewRoleRequest } from "~/bindings/CreateNewRoleRequest";
import { CreateNewRoleResponse } from "~/bindings/CreateNewRoleResponse";
import { ResourcePermissionType } from "~/bindings/ResourcePermissionType";
import { httpRequest } from "~/utils/http-request";
import WorkspaceHeader from "~/routes/_logged-in/_workspaced/workspace/-components/workspace-header";
import PermissionSelector from "./-components/permission-selector";
import { useFetchPermissions } from "~/hooks/fetch";
import { parsePermissionName, parseCamelCase } from "~/utils/func";
import { FiTrash2 } from "solid-icons/fi";

const CreateRoles = () => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();
	const toast = useToast();
	const navigate = useNavigate();

	const [roleName, setRoleName] = createSignal("");
	const [roleDescription, setRoleDescription] = createSignal("");
	const [permissionsData, setPermissionsData] = createSignal<{ [key: string]: ResourcePermissionType }>({});

	const [allPermissions] = useFetchPermissions(workspaceId()!);

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

	const resourceParamsWorkspace = () => {
		return [authState(), workspaceId()] as const;
	};

	const [workspaceInfo] = createResource(resourceParamsWorkspace, async ([auth, id]) => {
		if (!auth || auth.type !== "LoggedIn" || id === "") {
			return;
		}
		const response = await httpRequest<GetWorkspaceInfoResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${id}`,
			{
				method: "GET",
			}
		);
		if (!response.ok) {
			console.error("Failed to fetch workspace info:", response.data.error);
			toast("Failed to fetch workspace info", "error");
			return undefined;
		}
		return response.data;
	});

	const { execute: handleSubmit, isLoading: isSubmitting } = createAuthenticatedAction(async ({ workspaceId }) => {
		if (!roleName().trim()) {
			toast("Please enter a role name", "error");
			return;
		}

		if (Object.keys(permissionsData()).length === 0) {
			toast("Please select at least one permission", "error");
			return;
		}

		const requestBody: CreateNewRoleRequest = {
			name: roleName().trim(),
			description: roleDescription().trim() || `Role: ${roleName().trim()}`,
			permissions: permissionsData(),
		};

		const response = await httpRequest<CreateNewRoleResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${workspaceId}/rbac/role`,
			{
				method: "POST",
				body: JSON.stringify(requestBody),
			}
		);

		if (!response.ok) {
			console.error("Failed to create role:", response.data.error);
			toast(response.data.error || "Failed to create role", "error");
			return;
		}

		toast("Role created successfully", "success");
		navigate({ to: "/workspace/roles" });
	});

	return (
		<>
			<Title>New Role | Patr</Title>
			<PageContainer>
				<WorkspaceHeader workspaceName={workspaceInfo()?.name} activeTab="roles" />
				<PageContainerBody class="flex flex-col justify-between h-full gap-8">
					<div class="flex flex-col gap-6 flex-1">
						<div class="text-2xl text-white font-semibold">Create New Roles</div>

						<div class="flex flex-col gap-2">
							<label class="text-white text-sm">Role Name</label>
							<Input
								type="text"
								placeholder="Enter Name"
								value={roleName()}
								onInput={(e) => setRoleName(e.currentTarget.value)}
							/>
						</div>

						<div class="flex flex-col gap-2">
							<label class="text-white text-sm">Description</label>
							<Input
								type="text"
								placeholder="Enter Description (optional)"
								value={roleDescription()}
								onInput={(e) => setRoleDescription(e.currentTarget.value)}
							/>
						</div>

						<div class="flex flex-col gap-4">
							<div class="text-white text-sm font-medium">Permissions</div>
							<PermissionSelector
								workspaceId={workspaceId()!}
								onPermissionsDataChange={(data) => setPermissionsData((prev) => ({ ...prev, ...data }))}
							/>

							<Show when={permissionEntries().length > 0}>
								<Table
									column_grids={["flex-3", "flex-2", "flex-3", "flex-[0.5]"]}
									headings={["Resource Type", "Action", "Resources", ""]}
									rows={permissionEntries().sort(
										(a, b) =>
											a.resourceType.localeCompare(b.resourceType) ||
											a.action.localeCompare(b.action)
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
														{perm.resources.length} resource
														{perm.resources.length !== 1 ? "s" : ""}
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
							</Show>
						</div>
					</div>

					<div class="flex justify-end gap-4 border-t border-border-color pt-4">
						<Button
							variant={ButtonVariant.Outlined}
							onClick={() => navigate({ to: "/workspace/roles" })}
							disabled={isSubmitting()}
						>
							CANCEL
						</Button>
						<Button
							variant={ButtonVariant.Contained}
							onClick={() =>
								handleSubmit().catch(() => {
									toast("An unexpected error occurred while creating the role", "error");
								})
							}
							disabled={isSubmitting()}
						>
							{isSubmitting() ? "Creating..." : "Create Role"}
						</Button>
					</div>
				</PageContainerBody>
			</PageContainer>
		</>
	);
};

export const Route = createFileRoute("/_logged-in/_workspaced/workspace/roles/new")({
	component: CreateRoles,
});
