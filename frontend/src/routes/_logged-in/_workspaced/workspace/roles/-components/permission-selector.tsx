import { createMemo, createSignal, Show, Suspense } from "solid-js";
import { Button, ButtonVariant, InputDropdown, ListResources } from "~/components";
import { useFetchPermissions } from "~/hooks/fetch";
import { parsePermissionName, parseCamelCase, getResourceEndpoint } from "~/utils/func";
import { ResourcePermissionType } from "~/bindings/ResourcePermissionType";
import { FiPlus } from "solid-icons/fi";

interface PermissionSelectorProps {
	/** Additional classes for the container */
	class?: string;
	workspaceId: string;
	onPermissionsDataChange: (data: { [key: string]: ResourcePermissionType }) => void;
}

const PermissionSelector = (props: PermissionSelectorProps) => {
	const [selectedResourceType, setSelectedResourceType] = createSignal<string>("");
	const [selectedPermission, setSelectedPermission] = createSignal<string>("");
	const [scopeMode, setScopeMode] = createSignal<"all" | "include" | "exclude">("all");
	const [selectedResources, setSelectedResources] = createSignal<Set<string>>(new Set());

	const [permissions] = useFetchPermissions(props.workspaceId);

	// Get unique resource types from permissions
	const resourceTypeOptions = createMemo(() => {
		return Array.from(
			new Set(
				(permissions()?.permissions || []).map((p) => parsePermissionName(p.name).resourceType).filter((r) => r)
			)
		).map((resourceType) => ({
			label: parseCamelCase(resourceType),
			value: resourceType,
		}));
	});

	// Get permissions for the selected resource type
	const permissionsOptions = createMemo(() => {
		return (permissions()?.permissions || [])
			.filter((p) => parsePermissionName(p.name).resourceType === selectedResourceType())
			.map((p) => {
				const parsed = parsePermissionName(p.name);
				return {
					label: parseCamelCase(parsed.permission),
					value: parsed.permission,
				};
			});
	});

	// Find the permission ID for the selected resource type + permission
	const selectedPermissionId = createMemo(() => {
		const perms = permissions()?.permissions || [];
		const match = perms.find((p) => {
			const parsed = parsePermissionName(p.name);
			return parsed.resourceType === selectedResourceType() && parsed.permission === selectedPermission();
		});
		return match?.id;
	});

	// Whether the scope dropdown should be shown
	const shouldShowScope = createMemo(() => {
		if (!selectedResourceType() || !selectedPermission()) return false;
		if (!getResourceEndpoint(selectedResourceType())) return false;
		if (["create", "add"].includes(selectedPermission())) return false;
		return true;
	});

	const humanResourceType = createMemo(() => {
		if (selectedResourceType() == "containerRegistryRepository") return "Container Registry Repositories";
		return parseCamelCase(selectedResourceType()) + "s";
	});

	const toggleResource = (resourceId: string) => {
		const newSet = new Set(selectedResources());
		if (newSet.has(resourceId)) {
			newSet.delete(resourceId);
		} else {
			newSet.add(resourceId);
		}
		setSelectedResources(newSet);
	};

	const handleAdd = () => {
		const permId = selectedPermissionId();
		if (!permId || !selectedPermission()) return;

		let resourcePermission: ResourcePermissionType;
		if (!shouldShowScope() || scopeMode() === "all") {
			resourcePermission = { permissionType: "exclude", resources: [] };
		} else if (scopeMode() === "include") {
			resourcePermission = { permissionType: "include", resources: Array.from(selectedResources()) };
		} else {
			resourcePermission = { permissionType: "exclude", resources: Array.from(selectedResources()) };
		}

		props.onPermissionsDataChange({ [permId]: resourcePermission });

		// Reset dropdowns 2-4, keep dropdown 1
		setSelectedPermission("");
		setScopeMode("all");
		setSelectedResources(new Set([]));
	};

	return (
		<Suspense fallback={<div class="text-gray-400 text-sm">Loading permissions...</div>}>
			<div class={`flex gap-3 items-center ${props.class || ""}`}>
				{/* Dropdown 1: Resource Type */}
				<div class="flex-1 min-w-0">
					<InputDropdown
						onSelect={(val) => {
							setSelectedResourceType(val);
							setSelectedPermission("");
							setScopeMode("all");
							setSelectedResources(new Set([]));
						}}
						placeholder="Select Resource Type"
						value={selectedResourceType}
						options={resourceTypeOptions()}
					/>
				</div>

				{/* Dropdown 2: Action */}
				<Show when={selectedResourceType() && permissionsOptions().length > 0}>
					<div class="flex-1 min-w-0">
						<InputDropdown
							onSelect={(val) => {
								setSelectedPermission(val);
								setScopeMode("all");
								setSelectedResources(new Set([]));
							}}
							placeholder="Select Action"
							value={selectedPermission}
							options={permissionsOptions()}
						/>
					</div>
				</Show>

				{/* Dropdown 3: Resource Scope */}
				<Show when={selectedPermission() && shouldShowScope()}>
					<div class="flex-1 min-w-0">
						<InputDropdown
							onSelect={(val) => {
								setScopeMode(val as "all" | "include" | "exclude");
								if (val === "all") {
									setSelectedResources(new Set([]));
								}
							}}
							placeholder="Select Scope"
							value={scopeMode}
							options={[
								{ label: `All ${humanResourceType()}`, value: "all" },
								{ label: `Only Specific ${humanResourceType()}...`, value: "include" },
								{ label: `All ${humanResourceType()} Except...`, value: "exclude" },
							]}
						/>
					</div>
				</Show>

				{/* Dropdown 4: Resource Selection */}
				<Show when={selectedPermission() && shouldShowScope() && scopeMode() !== "all"}>
					<div class="flex-1 min-w-0">
						<ListResources
							workspaceId={props.workspaceId}
							resourceType={selectedResourceType()}
							selectedResources={selectedResources()}
							toggleResource={toggleResource}
						/>
					</div>
				</Show>

				{/* + Button */}
				<Button
					variant={ButtonVariant.Outlined}
					type="button"
					disabled={!selectedPermission()}
					onClick={handleAdd}
				>
					<FiPlus size={16} class="inline-block" />
				</Button>
			</div>
		</Suspense>
	);
};

export default PermissionSelector;
