import { createMemo, createResource, createSignal, For, Show } from "solid-js";
import { useNavigate, useParams } from "@solidjs/router";
import {
    Button,
    ButtonVariant,
    Input,
    PageContainer,
    PageContainerBody,
    useToast,
    WorkspaceHeader,
} from "~/components";
import { useAuthState } from "~/hooks";
import { GetWorkspaceInfoResponse } from "~/bindings/GetWorkspaceInfoResponse";
import { ListDeploymentResponse } from "~/bindings/ListDeploymentResponse";
import { ListAllPermissionsResponse } from "~/bindings/ListAllPermissionsResponse";
import { CreateNewRoleRequest } from "~/bindings/CreateNewRoleRequest";
import { CreateNewRoleResponse } from "~/bindings/CreateNewRoleResponse";
import { ResourcePermissionType } from "~/bindings/ResourcePermissionType";
import { httpRequest } from "~/utils/http-request";

interface PermissionCategory {
    title: string;
    permissionIds: string[];
}

const CreateRoles = () => {
    const params = useParams();
    const [authState] = useAuthState();
    const toast = useToast();
    const navigate = useNavigate();

    const [roleName, setRoleName] = createSignal("");
    const [roleDescription, setRoleDescription] = createSignal("");
    const [selectedPermissionIds, setSelectedPermissionIds] = createSignal<Set<string>>(
        new Set()
    );
    const [selectedResourceType, setSelectedResourceType] = createSignal<string>("");
    const [selectedDeployments, setSelectedDeployments] = createSignal<Set<string>>(
        new Set()
    );
    const [isSubmitting, setIsSubmitting] = createSignal(false);
    const [includeExcludeMode, setIncludeExcludeMode] = createSignal<"all" | "include" | "exclude">("all");

    // Helper to parse permission names like "deployment::view" into { resourceType: "deployment", action: "view" }
    const parsePermissionName = (name: string) => {
        const parts = name.split("::");
        return {
            resourceType: parts[0] || "",
            action: parts[1] || name,
        };
    };

    const resourceParamsWorkspace = () => {
        return [authState(), params.id] as const;
    };

    const [workspaceInfo] = createResource(
        resourceParamsWorkspace,
        async ([auth, id]) => {
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
        }
    );

    const fetchParams = createMemo(() => {
        return [authState(), params.id] as const;
    });

    const [deployments] = createResource(fetchParams, async ([auth, wsId]) => {
        if (!wsId || !auth || auth.type !== "LoggedIn") {
            return { deployments: [] };
        }

        const response = await httpRequest<ListDeploymentResponse>(
            `${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/deployment`,
            {
                method: "GET",
                headers: {
                    "Content-Type": "application/json",
                    Authorization: `Bearer ${auth.accessToken}`,
                },
            }
        );

        if (!response.ok) {
            console.error("Failed to fetch deployments:", response.data.error);
            toast("Failed to fetch deployments", "error");
            return { deployments: [] };
        }

        return response.data;
    });

    const [permissions] = createResource(fetchParams, async ([auth, wsId]) => {
        if (!wsId || !auth || auth.type !== "LoggedIn") {
            return { permissions: [] };
        }

        try {
            const response = await httpRequest<ListAllPermissionsResponse>(
                `${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/rbac/permission`,
                {
                    method: "GET",
                    headers: {
                        "Content-Type": "application/json",
                        Authorization: `Bearer ${auth.accessToken}`,
                    },
                }
            );

            if (!response.ok) {
                console.error("Failed to fetch permissions:", response.data.error);
                toast("Failed to fetch permissions. Please ensure permissions are properly configured in the database.", "error");
                return { permissions: [] };
            }

            return response.data;
        } catch (error) {
            console.error("Error fetching permissions:", error);
            toast("Failed to load permissions", "error");
            return { permissions: [] };
        }
    });

    // Permission categories - we'll dynamically populate these with actual permissions
    const permissionCategories: PermissionCategory[] = [
        {
            title: "Workspace",
            permissionIds: [],
        },
        {
            title: "Permissions",
            permissionIds: [],
        },
        {
            title: "Include/Exclude",
            permissionIds: [],
        },
    ];

    const togglePermissionId = (permissionId: string) => {
        const newSet = new Set(selectedPermissionIds());
        if (newSet.has(permissionId)) {
            newSet.delete(permissionId);
        } else {
            newSet.add(permissionId);
        }
        setSelectedPermissionIds(newSet);
    };

    const toggleDeployment = (deploymentId: string) => {
        const newSet = new Set(selectedDeployments());
        if (newSet.has(deploymentId)) {
            newSet.delete(deploymentId);
        } else {
            newSet.add(deploymentId);
        }
        setSelectedDeployments(newSet);
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
                } else if (mode === "include" && selectedDeployments().size > 0) {
                    permissions[permissionId] = {
                        permissionType: "include",
                        resources: Array.from(selectedDeployments()),
                    };
                } else if (mode === "exclude" && selectedDeployments().size > 0) {
                    permissions[permissionId] = {
                        permissionType: "exclude",
                        resources: Array.from(selectedDeployments()),
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

    return (
        <PageContainer>
            <WorkspaceHeader
                workspaceName={workspaceInfo()?.name}
                activeTab="roles"
            />
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

                        <Show
                            when={!permissions.loading && permissions()}
                            fallback={<div class="text-gray-400 text-sm">Loading permissions...</div>}
                        >
                            <div class="grid grid-cols-4 gap-6 mb-4">
                                {/* Column 1: Resource Types */}
                                <div class="flex flex-col gap-3">
                                    <div class="text-white font-medium border-b border-border-color pb-2">
                                        Resource Type
                                    </div>
                                    <div class="flex flex-col gap-2.5">
                                        <For each={Array.from(new Set((permissions()?.permissions || []).map(p => parsePermissionName(p.name).resourceType).filter(r => r)))}>
                                            {(resourceType) => (
                                                <label class="flex items-center gap-2 cursor-pointer">
                                                    <input
                                                        type="radio"
                                                        name="resourceType"
                                                        checked={selectedResourceType() === resourceType}
                                                        onChange={() => setSelectedResourceType(resourceType)}
                                                        class="w-4 h-4"
                                                    />
                                                    <span class="text-white text-sm capitalize">
                                                        {resourceType}
                                                    </span>
                                                </label>
                                            )}
                                        </For>
                                    </div>
                                </div>

                                {/* Column 2: Permission Actions (filtered by selected resource type) */}
                                <div class="flex flex-col gap-3">
                                    <div class="text-white font-medium border-b border-border-color pb-2">
                                        Permissions
                                    </div>
                                    <div class="flex flex-col gap-2.5">
                                        <Show when={selectedResourceType()}>
                                            <For each={(permissions()?.permissions || []).filter(p => {
                                                const parsed = parsePermissionName(p.name);
                                                return parsed.resourceType === selectedResourceType();
                                            })}>
                                                {(permission) => {
                                                    const parsed = parsePermissionName(permission.name);
                                                    return (
                                                        <label class="flex items-center gap-2 cursor-pointer">
                                                            <input
                                                                type="checkbox"
                                                                checked={selectedPermissionIds().has(permission.id)}
                                                                onChange={() => togglePermissionId(permission.id)}
                                                                class="w-4 h-4 rounded border-border-color bg-secondary-light checked:bg-primary"
                                                            />
                                                            <span class="text-white text-sm">
                                                                {parsed.action}
                                                            </span>
                                                        </label>
                                                    );
                                                }}
                                            </For>
                                        </Show>
                                        <Show when={!selectedResourceType()}>
                                            <span class="text-gray-400 text-sm">Select a resource type first</span>
                                        </Show>
                                    </div>
                                </div>

                                {/* Column 3: Include/Exclude */}
                                <div class="flex flex-col gap-3">
                                    <div class="text-white font-medium border-b border-border-color pb-2">
                                        Include/Exclude
                                    </div>
                                    <div class="flex flex-col gap-2.5">
                                        <label class="flex items-center gap-2 cursor-pointer">
                                            <input
                                                type="radio"
                                                name="includeExclude"
                                                checked={includeExcludeMode() === "all"}
                                                onChange={() => setIncludeExcludeMode("all")}
                                                class="w-4 h-4"
                                            />
                                            <span class="text-white text-sm">All deployment</span>
                                        </label>
                                        <label class="flex items-center gap-2 cursor-pointer">
                                            <input
                                                type="radio"
                                                name="includeExclude"
                                                checked={includeExcludeMode() === "include"}
                                                onChange={() => setIncludeExcludeMode("include")}
                                                class="w-4 h-4"
                                            />
                                            <span class="text-white text-sm">Include deployment</span>
                                        </label>
                                        <label class="flex items-center gap-2 cursor-pointer">
                                            <input
                                                type="radio"
                                                name="includeExclude"
                                                checked={includeExcludeMode() === "exclude"}
                                                onChange={() => setIncludeExcludeMode("exclude")}
                                                class="w-4 h-4"
                                            />
                                            <span class="text-white text-sm">Exclude deployment</span>
                                        </label>
                                    </div>
                                </div>

                                {/* Column 4: List of Deployments */}
                                <div class="flex flex-col gap-3">
                                    <div class="text-white font-medium border-b border-border-color pb-2">
                                        List of deployment
                                    </div>
                                    <div class="flex flex-col gap-2.5 max-h-[300px] overflow-y-auto">
                                        <Show when={includeExcludeMode() !== "all"}>
                                            {typeof window !== "undefined" && (
                                                <Show
                                                    when={!deployments.loading && deployments()}
                                                    fallback={<span class="text-gray-400 text-sm">Loading...</span>}
                                                >
                                                    <Show
                                                        when={deployments()?.deployments && deployments()!.deployments.length > 0}
                                                        fallback={<span class="text-gray-400 text-sm">No deployments</span>}
                                                    >
                                                        <For each={deployments()!.deployments}>
                                                            {(deployment) => (
                                                                <label class="flex items-center gap-2 cursor-pointer">
                                                                    <input
                                                                        type="checkbox"
                                                                        checked={selectedDeployments().has(deployment.id)}
                                                                        onChange={() => toggleDeployment(deployment.id)}
                                                                        class="w-4 h-4 rounded border-border-color bg-secondary-light checked:bg-primary"
                                                                    />
                                                                    <span class="text-white text-sm truncate">
                                                                        {deployment.name}
                                                                    </span>
                                                                </label>
                                                            )}
                                                        </For>
                                                    </Show>
                                                </Show>
                                            )}
                                        </Show>
                                        <Show when={includeExcludeMode() === "all"}>
                                            <span class="text-gray-400 text-sm">Select include/exclude first</span>
                                        </Show>
                                    </div>
                                </div>
                            </div>
                        </Show>
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
                    <Button
                        variant={ButtonVariant.Contained}
                        onClick={handleSubmit}
                        disabled={isSubmitting()}
                    >
                        {isSubmitting() ? "CREATING..." : "CONFIRM"}
                    </Button>
                </div>
            </PageContainerBody>
        </PageContainer>
    );
};

export default CreateRoles;
