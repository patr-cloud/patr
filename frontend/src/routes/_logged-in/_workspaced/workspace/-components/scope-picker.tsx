import { createMemo, createSignal, For, Show } from "solid-js";
import { FiX } from "solid-icons/fi";
import { InputDropdown, ListResources } from "~/components";
import { PermissionScope } from "~/bindings/PermissionScope";
import { useResourcesInfoQuery, useRoleInfoQuery, usePermissionsQuery } from "~/hooks/fetch";
import { getResourceEndpoint, parseCamelCase, parsePermissionName } from "~/utils/func";

interface ScopePickerProps {
	workspaceId: string;
	/** The role whose grant is being scoped — limits the resource types offered. */
	roleId: string;
	scope: PermissionScope;
	onChange: (scope: PermissionScope) => void;
}

/**
 * Edits where one role grant applies: the entire workspace, or an explicit set
 * of resources. The resource picker only offers the types the role's
 * permissions actually touch — scoping a role to resources it has no
 * permission on would grant nothing.
 */
const ScopePicker = (props: ScopePickerProps) => {
	const [selectedResourceType, setSelectedResourceType] = createSignal<string>("");

	const roleInfoQuery = useRoleInfoQuery(() => props.roleId);
	const permissionsQuery = usePermissionsQuery(() => props.workspaceId);

	// The listable resource types this role's permissions touch. Workspace-level
	// permissions (viewRoles etc.) have no resources to scope to, so a role of
	// only those offers nothing here.
	const applicableResourceTypes = createMemo(() => {
		const granted = new Set(roleInfoQuery.data?.permissions ?? []);
		const types = (permissionsQuery.data?.permissions ?? [])
			.filter((permission) => granted.has(permission.id))
			.map((permission) => parsePermissionName(permission.name).resourceType)
			.filter((resourceType) => resourceType && getResourceEndpoint(resourceType));
		return Array.from(new Set(types));
	});

	const selectedResources = createMemo(() => (props.scope.scopeType === "resources" ? props.scope.resources : []));

	// Resolve ids to names for the chips; unresolvable ids fall back to the raw id.
	const resourcesInfoQuery = useResourcesInfoQuery(() => selectedResources());

	const toggleResource = (resourceId: string) => {
		const current = selectedResources();
		const next = current.includes(resourceId)
			? current.filter((id) => id !== resourceId)
			: [...current, resourceId];
		props.onChange({ scopeType: "resources", resources: next });
	};

	return (
		<div class="flex flex-col gap-2">
			{/* Stacked, not side-by-side — this renders inside the narrow member
			  detail panel where three dropdowns in a row don't fit. */}
			<div class="flex flex-col gap-2">
				<div class="flex-1 min-w-0">
					<InputDropdown
						onSelect={(mode) =>
							props.onChange(
								mode === "workspace"
									? { scopeType: "workspace" }
									: { scopeType: "resources", resources: selectedResources() }
							)
						}
						placeholder="Select scope"
						value={() => props.scope.scopeType}
						options={[
							{ label: "Entire workspace", value: "workspace" },
							{ label: "Specific resources...", value: "resources" },
						]}
					/>
				</div>
				<Show when={props.scope.scopeType === "resources"}>
					<div class="flex-1 min-w-0">
						<InputDropdown
							onSelect={setSelectedResourceType}
							placeholder="Resource type"
							value={selectedResourceType}
							options={applicableResourceTypes().map((resourceType) => ({
								label: parseCamelCase(resourceType),
								value: resourceType,
							}))}
						/>
					</div>
					<Show when={selectedResourceType()}>
						<div class="flex-1 min-w-0">
							<ListResources
								workspaceId={props.workspaceId}
								resourceType={selectedResourceType()}
								selectedResources={new Set(selectedResources())}
								toggleResource={toggleResource}
							/>
						</div>
					</Show>
				</Show>
			</div>
			<Show when={props.scope.scopeType === "resources"}>
				<Show
					when={selectedResources().length > 0}
					fallback={
						<p class="text-warning text-xs">No resources selected — this grant applies to nothing.</p>
					}
				>
					<div class="flex flex-wrap gap-1.5">
						<For each={selectedResources()}>
							{(resourceId) => (
								<span class="inline-flex items-center gap-1.5 px-2 py-0.5 bg-secondary border border-border-color rounded-xs text-white text-xs">
									{resourcesInfoQuery.data?.get(resourceId)?.name || resourceId}
									<button
										type="button"
										aria-label="Remove resource from scope"
										onClick={() => toggleResource(resourceId)}
										class="text-grey hover:text-error transition-colors cursor-pointer"
									>
										<FiX size={11} />
									</button>
								</span>
							)}
						</For>
					</div>
				</Show>
			</Show>
		</div>
	);
};

export default ScopePicker;
