import { createMemo, createSignal, Show, Suspense } from "solid-js";
import { Button, ButtonVariant, InputDropdown, ListResources } from "~/components";
import { usePermissionsQuery } from "~/hooks/fetch";
import { parsePermissionName, parseCamelCase, getResourceEndpoint, workspaceLevelResourceTypes } from "~/utils/func";
import { ResourcePermissionType } from "~/bindings/ResourcePermissionType";
import { FiPlus } from "solid-icons/fi";

interface PermissionSelectorProps {
	/** Additional classes for the container */
	class?: string;
	workspaceId: string;
	/**
	 * The permissions currently granted, keyed by permission ID. Selecting a
	 * permission that is already in here seeds the scope and resource pickers with
	 * its existing value, so that adding a resource extends the set instead of
	 * replacing it.
	 */
	permissionsData: { [key: string]: ResourcePermissionType };
	onPermissionsDataChange: (data: { [key: string]: ResourcePermissionType }) => void;
}

const PermissionSelector = (props: PermissionSelectorProps) => {
	const [selectedResourceType, setSelectedResourceType] = createSignal<string>("");
	const [selectedPermission, setSelectedPermission] = createSignal<string>("");
	const [scopeMode, setScopeMode] = createSignal<"all" | "include" | "exclude">("all");
	const [selectedResources, setSelectedResources] = createSignal<Set<string>>(new Set());

	const permissionsQuery = usePermissionsQuery(() => props.workspaceId);

	// Get unique resource types from permissions
	const resourceTypeOptions = createMemo(() => {
		return Array.from(
			new Set(
				(permissionsQuery.data?.permissions || [])
					.map((p) => parsePermissionName(p.name).resourceType)
					.filter((r) => r)
			)
		).map((resourceType) => ({
			label: parseCamelCase(resourceType),
			value: resourceType,
		}));
	});

	// Get permissions for the selected resource type
	const permissionsOptions = createMemo(() => {
		return (permissionsQuery.data?.permissions || [])
			.filter((p) => parsePermissionName(p.name).resourceType === selectedResourceType())
			.map((p) => {
				const parsed = parsePermissionName(p.name);
				return {
					label: parseCamelCase(parsed.permission),
					value: parsed.permission,
				};
			});
	});

	// Whether the selected resource type is workspace-level (no per-resource actions)
	const isWorkspaceLevelSelected = createMemo(() => workspaceLevelResourceTypes.has(selectedResourceType()));

	// Find the permission ID for the selected resource type + permission
	const selectedPermissionId = createMemo(() => {
		const perms = permissionsQuery.data?.permissions || [];
		if (isWorkspaceLevelSelected()) {
			// Workspace-level types have no action — match by resourceType only
			const match = perms.find((p) => {
				const parsed = parsePermissionName(p.name);
				return parsed.resourceType === selectedResourceType();
			});
			return match?.id;
		}
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
		if (!permId) return;

		// Workspace-level types don't need an action selection
		if (!isWorkspaceLevelSelected() && !selectedPermission()) return;

		let resourcePermission: ResourcePermissionType;
		if (isWorkspaceLevelSelected() || !shouldShowScope() || scopeMode() === "all") {
			// Choosing "All X" is a deliberate widening, so it clears any existing scoping.
			resourcePermission = { permissionType: "exclude", resources: [] };
		} else {
			const permissionType = scopeMode() === "include" ? "include" : "exclude";
			const picked = Array.from(selectedResources());
			const existing = props.permissionsData[permId];

			// The parent applies this value wholesale, so adding to a permission that
			// already lists resources has to carry the existing ones through — otherwise
			// they'd be silently dropped. Only union within the same scope type: going
			// from "only these" to "all except these" inverts the meaning, so an
			// explicit switch replaces rather than merges.
			const resources =
				existing && existing.permissionType === permissionType
					? Array.from(new Set([...existing.resources, ...picked]))
					: picked;

			resourcePermission = { permissionType, resources };
		}

		props.onPermissionsDataChange({ [permId]: resourcePermission });

		// Reset dropdowns 2-4, keep dropdown 1
		setSelectedPermission("");
		setScopeMode("all");
		setSelectedResources(new Set([]));
	};

	return (
		<Suspense fallback={<div class="text-gray-400 text-sm">Loading permissions...</div>}>
			<div class={`flex flex-col md:flex-row gap-3 md:items-center ${props.class || ""}`}>
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

				{/* Dropdown 2: Action (hidden for workspace-level types) */}
				<Show when={!isWorkspaceLevelSelected() && selectedResourceType() && permissionsOptions().length > 0}>
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

				{/* Dropdown 3: Resource Scope (hidden for workspace-level types) */}
				<Show when={!isWorkspaceLevelSelected() && selectedPermission() && shouldShowScope()}>
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

				{/* Dropdown 4: Resource Selection (hidden for workspace-level types) */}
				<Show
					when={
						!isWorkspaceLevelSelected() &&
						selectedPermission() &&
						shouldShowScope() &&
						scopeMode() !== "all"
					}
				>
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
					aria-label="Add Permission"
					disabled={!isWorkspaceLevelSelected() && !selectedPermission()}
					onClick={handleAdd}
				>
					<FiPlus size={16} class="inline-block" />
				</Button>
			</div>
		</Suspense>
	);
};

export default PermissionSelector;
