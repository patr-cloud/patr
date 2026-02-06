import { createResource, createSignal } from "solid-js";
import { useNavigate, useParams } from "@solidjs/router";
import {
	Button,
	ButtonVariant,
	Input,
	PageContainer,
	PageContainerBody,
	useToast,
} from "~/components";
import { useAuthState } from "~/hooks";
import { GetWorkspaceInfoResponse } from "~/bindings/GetWorkspaceInfoResponse";
import { CreateNewRoleRequest } from "~/bindings/CreateNewRoleRequest";
import { CreateNewRoleResponse } from "~/bindings/CreateNewRoleResponse";
import { ResourcePermissionType } from "~/bindings/ResourcePermissionType";
import { httpRequest } from "~/utils/http-request";
import WorkspaceHeader from "~/pages/workspace/workspace-header";
import PermissionSelector from "./permission-selector";

const CreateRoles = () => {
	const params = useParams();
	if (!params.id) {
		throw new Error("Workspace ID is required in the URL parameters");
	}
	const [authState] = useAuthState();
	const toast = useToast();
	const navigate = useNavigate();

	const [roleName, setRoleName] = createSignal("");
	const [roleDescription, setRoleDescription] = createSignal("");
	const [selectedPermissionIds, setSelectedPermissionIds] = createSignal<Set<string>>(new Set());
	const [permissionsData, setPermissionsData] = createSignal<{ [key: string]: ResourcePermissionType }>({});
	const [isSubmitting, setIsSubmitting] = createSignal(false);

	const resourceParamsWorkspace = () => {
		return [authState(), params.id] as const;
	};

	const [workspaceInfo] = createResource(resourceParamsWorkspace, async ([auth, id]) => {
		if (!auth || auth.type !== "LoggedIn" || id === "") {
			return;
		}
		const response = await httpRequest<GetWorkspaceInfoResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${id}`,
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
			return undefined;
		}
		return response.data;
	});

	const handleSubmit = async () => {
		if (!roleName().trim()) {
			toast("Please enter a role name", "error");
			return;
		}

		if (selectedPermissionIds().size === 0) {
			toast("Please select at least one permission", "error");
			return;
		}

		const auth = authState();
		if (!auth || auth.type !== "LoggedIn") {
			toast("You must be logged in to create a role", "error");
			return;
		}

		setIsSubmitting(true);

		try {
			const requestBody: CreateNewRoleRequest = {
				name: roleName().trim(),
				description: roleDescription().trim() || `Role: ${roleName().trim()}`,
				permissions: permissionsData(),
			};

			const response = await httpRequest<CreateNewRoleResponse>(
				`${import.meta.env.VITE_BASE_URL}/api/workspace/${params.id}/rbac/role`,
				{
					method: "POST",
					headers: {
						"Content-Type": "application/json",
						Authorization: `Bearer ${auth.accessToken}`,
					},
					body: JSON.stringify(requestBody),
				}
			);

			if (!response.ok) {
				console.error("Failed to create role:", response.data.error);
				toast(response.data.error || "Failed to create role", "error");
				return;
			}

			toast("Role created successfully", "success");
			navigate(`/workspaces/${params.id}/roles`);
		} catch (error) {
			console.error("Error creating role:", error);
			toast("An error occurred while creating the role", "error");
		} finally {
			setIsSubmitting(false);
		}
	};

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
							workspaceId={params.id}
							selectedPermissionIds={selectedPermissionIds()}
							onPermissionChange={setSelectedPermissionIds}
							onPermissionsDataChange={setPermissionsData}
						/>
					</div>
				</div>

				<div class="flex justify-end gap-4 border-t border-border-color pt-4">
					<Button
						variant={ButtonVariant.Outlined}
						onClick={() => navigate(`/workspaces/${params.id}/roles`)}
						disabled={isSubmitting()}
					>
						CANCEL
					</Button>
					<Button variant={ButtonVariant.Contained} onClick={handleSubmit} disabled={isSubmitting()}>
						{isSubmitting() ? "CREATING..." : "CONFIRM"}
					</Button>
				</div>
			</PageContainerBody>
		</PageContainer>
	);
};

export default CreateRoles;
