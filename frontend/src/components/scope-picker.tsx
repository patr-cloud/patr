import { createMemo, For, Show, Suspense } from "solid-js";
import InputDropdown from "./input-dropdown";
import ListResources from "./list-resources";
import { Scope } from "~/utils/scope";
import { useResourcesInfoQuery, useRoleInfoQuery, usePermissionsQuery } from "~/hooks/fetch";
import { getResourceEndpoint, parsePermissionName } from "~/utils/func";

interface ScopePickerProps {
	workspaceId: string;
	/**
	 * The role whose grant is being scoped. Its permissions limit the resource
	 * types offered. Omit when scoping something that is not a role, and pass
	 * [permissionIds] instead.
	 */
	roleId?: string;
	/**
	 * The permissions being scoped directly, for grants that carry permissions
	 * rather than a role (API token ceilings). Takes precedence over [roleId],
	 * and avoids the role lookup entirely — reading a workspace's roles is
	 * permission-gated, and a token's owner may not hold that permission.
	 */
	permissionIds?: readonly string[];
	scope: Scope;
	onChange: (scope: Scope) => void;
	/**
	 * `stacked` fits a narrow column (the token screens); `inline` lays the
	 * dropdowns out across a full-width binding row.
	 */
	orientation?: "stacked" | "inline";
	/** The colour variant of the dropdowns. */
	styleVariant?: "light" | "medium" | "dark";
}

/**
 * Edits where one grant applies: the entire workspace, or an explicit set of
 * resources. The resource picker only offers the types the granted permissions
 * actually touch — scoping to resources they have no permission on would grant
 * nothing.
 */
const ScopePicker = (props: ScopePickerProps) => {
	// Disabled when there is no role: the query gates on a truthy id.
	const roleInfoQuery = useRoleInfoQuery(
		() => props.roleId ?? "",
		() => props.workspaceId
	);
	const permissionsQuery = usePermissionsQuery(() => props.workspaceId);

	/**
	 * Reads a query's data without suspending on it.
	 *
	 * Touching `.data` while a query is pending is what hands control to the
	 * nearest Suspense boundary. Both queries below are cold the first time the
	 * editor opens for a workspace or a role, so reading them directly meant
	 * every "Edit access" click blanked the rows for a round trip. Checking
	 * status first is not a suspense-tracked read, so this returns undefined
	 * and re-runs when the data lands — the editor draws immediately and the
	 * type list fills in underneath it.
	 */
	const settled = <T,>(query: { data: T | undefined; isSuccess: boolean }): T | undefined =>
		query.isSuccess ? query.data : undefined;

	// The listable resource types the granted permissions touch. Workspace-level
	// permissions (viewRoles etc.) have no resources to scope to, so a grant of
	// only those offers nothing here.
	const applicableResourceTypes = createMemo(() => {
		const granted = new Set(props.permissionIds ?? settled(roleInfoQuery)?.permissions ?? []);
		const types = (settled(permissionsQuery)?.permissions ?? [])
			.filter((permission) => granted.has(permission.id))
			.map((permission) => parsePermissionName(permission.name).resourceType)
			.filter((resourceType) => resourceType && getResourceEndpoint(resourceType));
		return Array.from(new Set(types));
	});

	const selectedResources = createMemo(() => (props.scope.scopeType === "resources" ? props.scope.resources : []));

	// Resolves which type each selected id is, so each picker can count its own.
	const resourcesInfoQuery = useResourcesInfoQuery(
		() => selectedResources(),
		() => props.workspaceId
	);

	/** How many selected resources are of `resourceType`, for that picker's placeholder. */
	const selectedCount = (resourceType: string) => {
		// With one picker every selected id is its type; no lookup needed.
		if (applicableResourceTypes().length === 1) return selectedResources().length;
		const info = settled(resourcesInfoQuery);
		return selectedResources().filter((resourceId) => info?.get(resourceId)?.resourceType === resourceType).length;
	};

	const toggleResource = (resourceId: string) => {
		const current = selectedResources();
		const next = current.includes(resourceId)
			? current.filter((id) => id !== resourceId)
			: [...current, resourceId];
		props.onChange({ scopeType: "resources", resources: next });
	};

	const isInline = () => props.orientation === "inline";

	return (
		// Own boundary. Every dropdown in here reads a query that re-keys as the
		// user edits (role info, resource lists, resource names); without this the
		// suspension escapes to whatever boundary the page happens to have and
		// blanks the entire screen on each click.
		<Suspense
			fallback={
				<div class="h-9 flex items-center text-grey text-xs">
					<span>Loading scope...</span>
				</div>
			}
		>
			<div class="flex flex-col gap-2">
				<div class={isInline() ? "flex flex-col sm:flex-row gap-2 items-start" : "flex flex-col gap-2"}>
					<div class="flex-1 min-w-0 w-full">
						<InputDropdown
							styleVariant={props.styleVariant}
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
					{/* One picker per resource type the grant touches. The scope is one
					  flat id list, so each picker just toggles its own type's ids in it. */}
					<Show when={props.scope.scopeType === "resources"}>
						<For each={applicableResourceTypes()}>
							{(resourceType) => (
								<div class="flex-1 min-w-0 w-full">
									<ListResources
										styleVariant={props.styleVariant}
										workspaceId={props.workspaceId}
										resourceType={resourceType}
										selectedResources={new Set(selectedResources())}
										toggleResource={toggleResource}
										placeholder={
											selectedCount(resourceType) > 0
												? `${selectedCount(resourceType)} selected`
												: undefined
										}
									/>
								</div>
							)}
						</For>
					</Show>
				</div>
				<Show when={props.scope.scopeType === "resources" && selectedResources().length === 0}>
					<p class="text-warning text-xs">No resources selected — this grant applies to nothing.</p>
				</Show>
			</div>
		</Suspense>
	);
};

export default ScopePicker;
