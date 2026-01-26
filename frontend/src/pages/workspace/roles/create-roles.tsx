import { createMemo, createResource, createSignal, For, Show, Suspense } from "solid-js";
import { useNavigate, useParams } from "@solidjs/router";
import {
	Button,
	ButtonVariant,
	Input,
	InputDropdown,
	InputDropdownCheckBox,
	ListResources,
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
import useFetchPermissions from "../../../hooks/use-fetch/use-fetch-permissions";
import { parsePermissionName, parseCamelCase, getResourceEndpoint } from "~/utils/func";

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
	const [selectedResourceType, setSelectedResourceType] = createSignal<string>("");
	const [selectedResources, setSelectedResources] = createSignal<Set<string>>(new Set());
	const [isSubmitting, setIsSubmitting] = createSignal(false);
	const [includeExcludeMode, setIncludeExcludeMode] = createSignal<"all" | "include" | "exclude">("all");

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

	const [permissions] = useFetchPermissions(params.id);

	const togglePermissionId = (permissionId: string) => {
		const newSet = new Set(selectedPermissionIds());
		if (newSet.has(permissionId)) {
			newSet.delete(permissionId);
		} else {
			newSet.add(permissionId);
		}
		setSelectedPermissionIds(newSet);
	};

	const toggleResource = (resourceId: string) => {
		const newSet = new Set(selectedResources());
		if (newSet.has(resourceId)) {
			newSet.delete(resourceId);
		} else {
			newSet.add(resourceId);
		}
		setSelectedResources(newSet);
	};

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
			// Build permissions object using actual permission IDs
			const permissions: { [key: string]: ResourcePermissionType } = {};

			selectedPermissionIds().forEach((permissionId) => {
				// Determine permission type based on mode and selected deployments
				const mode = includeExcludeMode();

				if (mode === "all") {
					permissions[permissionId] = {
						permissionType: "include",
						resources: [],
					};
				} else if (mode === "include" && selectedResources().size > 0) {
					permissions[permissionId] = {
						permissionType: "include",
						resources: Array.from(selectedResources()),
					};
				} else if (mode === "exclude" && selectedResources().size > 0) {
					permissions[permissionId] = {
						permissionType: "exclude",
						resources: Array.from(selectedResources()),
					};
				} else {
					permissions[permissionId] = {
						permissionType: "include",
						resources: [],
					};
				}
			});

			const requestBody: CreateNewRoleRequest = {
				name: roleName().trim(),
				description: roleDescription().trim() || `Role: ${roleName().trim()}`,
				permissions: permissions,
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

	const permissionActions = createMemo(() => {
		return (permissions()?.permissions || []).filter((p) => {
			const parsed = parsePermissionName(p.name);
			return parsed.action !== "" ? parsed.resourceType === selectedResourceType() : null;
		});
	});

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

						<Suspense fallback={<div class="text-gray-400 text-sm">Loading permissions...</div>}>
							<div class="flex gap-3">
								{/* column 1: resource types */}
								<div class="flex flex-col gap-3 w-full">
									<InputDropdown
										onSelect={(val) => {
											console.log(val);
											setSelectedResourceType(val);
											setIncludeExcludeMode("all");
											setSelectedPermissionIds(new Set<string>([]));
										}}
										placeholder="Select Resource Type"
										value={selectedResourceType}
										options={Array.from(
											new Set(
												(permissions()?.permissions || [])
													.map((p) => parsePermissionName(p.name).resourceType)
													.filter((r) => r)
											)
										).map((resourceType) => ({
											label: parseCamelCase(resourceType),
											value: resourceType,
										}))}
									/>
								</div>

								{/* Column 2: Permission Actions (only visible if permissions available) */}
								<Show when={selectedResourceType() && permissionActions().length > 0}>
									<div class="flex flex-col gap-3 w-full">
										<InputDropdownCheckBox
											onToggle={(val) => togglePermissionId(val)}
											checked={() => Array.from(selectedPermissionIds())}
											placeholder={() =>
												Array.from(selectedPermissionIds())
													.map((s) => permissionActions().find((p) => p.id === s)?.name)
													.map((val) => (val ? parseCamelCase(parsePermissionName(val).action) : undefined))
													.join(", ") || "Select Permissions"
											}
											options={() =>
												permissionActions().map((p) => {
													const parsed = parsePermissionName(p.name);
													return {
														label: `${parseCamelCase(parsed.action)}`,
														value: p.id,
													};
												})
											}
										/>
									</div>
								</Show>

								{/* Column 3: Include/Exclude (visible when resource type is selected) */}
								<Show when={selectedResourceType() && getResourceEndpoint(selectedResourceType())}>
									<div class="flex flex-col gap-3 w-full">
										<InputDropdown
											onSelect={(val) => setIncludeExcludeMode(val as "all" | "include" | "exclude")}
											placeholder="Select Include/Exclude Mode"
											value={includeExcludeMode}
											options={[
												{
													label: `All ${parseCamelCase(selectedResourceType())}(s)`,
													value: "all",
												},
												{
													label: `Include Specific ${parseCamelCase(selectedResourceType())}(s)`,
													value: "include",
												},
												{
													label: `Exclude Specific ${parseCamelCase(selectedResourceType())}(s)`,
													value: "exclude",
												},
											]}
										/>
									</div>
								</Show>

								{/* Column 4: List of Resources */}
								<Show
									when={
										selectedResourceType() &&
										includeExcludeMode() !== "all" &&
										getResourceEndpoint(selectedResourceType())
									}
								>
									<div class="flex flex-col gap-3 w-full">
										<ListResources
											workspaceId={params.id}
											resourceType={selectedResourceType()}
											selectedResources={selectedResources()}
											toggleResource={toggleResource}
										/>
									</div>
								</Show>
							</div>
						</Suspense>
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
