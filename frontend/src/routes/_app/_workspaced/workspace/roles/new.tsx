import { createFileRoute } from "@tanstack/solid-router";
import { createResource, createSignal } from "solid-js";
import { useNavigate } from "@tanstack/solid-router";
import { Button, ButtonVariant, Input, PageContainer, PageContainerBody, useToast } from "~/components";
import { createAuthenticatedAction, useAuthState } from "~/hooks";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { GetWorkspaceInfoResponse } from "~/bindings/GetWorkspaceInfoResponse";
import { CreateNewRoleRequest } from "~/bindings/CreateNewRoleRequest";
import { CreateNewRoleResponse } from "~/bindings/CreateNewRoleResponse";
import { ResourcePermissionType } from "~/bindings/ResourcePermissionType";
import { httpRequest } from "~/utils/http-request";
import WorkspaceHeader from "../-components/workspace-header";
import PermissionSelector from "./-components/permission-selector";

const CreateRoles = () => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();
	const toast = useToast();
	const navigate = useNavigate();

	const [roleName, setRoleName] = createSignal("");
	const [roleDescription, setRoleDescription] = createSignal("");
	const [selectedPermissionIds, setSelectedPermissionIds] = createSignal<Set<string>>(new Set());
	const [permissionsData, setPermissionsData] = createSignal<{ [key: string]: ResourcePermissionType }>({});

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

	const { execute: handleSubmit, isLoading: isSubmitting } = createAuthenticatedAction(
		async ({ accessToken, workspaceId }) => {
			if (!roleName().trim()) {
				toast("Please enter a role name", "error");
				return;
			}

			if (selectedPermissionIds().size === 0) {
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
		}
	);

	return (
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
							selectedPermissionIds={selectedPermissionIds()}
							onPermissionChange={setSelectedPermissionIds}
							onPermissionsDataChange={setPermissionsData}
						/>
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
	);
};

export const Route = createFileRoute("/_app/_workspaced/workspace/roles/new")({
	component: CreateRoles,
});
