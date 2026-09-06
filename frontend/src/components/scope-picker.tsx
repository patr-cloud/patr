import { createEffect, createMemo, createSignal, For, Show, Suspense } from "solid-js";
import { FiX } from "solid-icons/fi";
import InputDropdown from "./input-dropdown";
import ListResources from "./list-resources";
import { Scope } from "~/utils/scope";
import { useResourcesInfoQuery, useRoleInfoQuery, usePermissionsQuery } from "~/hooks/fetch";
import { getResourceEndpoint, parseCamelCase, parsePermissionName } from "~/utils/func";

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
}

/**
 * Edits where one grant applies: the entire workspace, or an explicit set of
 * resources. The resource picker only offers the types the granted permissions
 * actually touch — scoping to resources they have no permission on would grant
 * nothing.
 */
const ScopePicker = (props: ScopePickerProps) => {
	const [selectedResourceType, setSelectedResourceType] = createSignal<string>("");

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

	// Resolve ids to names for the chips; unresolvable ids fall back to the raw id.
	const resourcesInfoQuery = useResourcesInfoQuery(
		() => selectedResources(),
		() => props.workspaceId
	);

	/**
	 * True once the lookup has actually answered for the current selection.
	 * The placeholder that keeps this query from suspending is an empty map, so
	 * "success" alone is not enough to conclude an id has no resource.
	 */
	const namesResolved = () => resourcesInfoQuery.isSuccess && !resourcesInfoQuery.isPlaceholderData;

	/**
	 * The selection split by resource type, in the order the types are offered
	 * above.
	 *
	 * Ids the lookup hasn't answered for yet go to a trailing group rendered as
	 * blank placeholders — never as their raw id. Showing the id is what made
	 * opening the editor flash: every chip appeared ungrouped as a UUID for one
	 * round trip, then rearranged into its real type. An id still unknown
	 * *after* the lookup settles is a different thing — a deleted resource, or
	 * one from another workspace — and that genuinely belongs under "Unknown".
	 */
	const groupedSelection = createMemo(() => {
		const info = settled(resourcesInfoQuery);
		const resolved = namesResolved();
		const byType = new Map<string, string[]>();
		const unresolved: string[] = [];

		for (const resourceId of selectedResources()) {
			const resourceType = info?.get(resourceId)?.resourceType;
			if (resourceType) {
				byType.set(resourceType, [...(byType.get(resourceType) ?? []), resourceId]);
			} else if (resolved) {
				byType.set("", [...(byType.get("") ?? []), resourceId]);
			} else {
				unresolved.push(resourceId);
			}
		}

		const ordered = applicableResourceTypes().filter((resourceType) => byType.has(resourceType));
		const rest = [...byType.keys()].filter((resourceType) => !ordered.includes(resourceType));
		const groups = [...ordered, ...rest].map((resourceType) => ({
			label: resourceType ? parseCamelCase(resourceType) : "Unknown",
			resourceIds: byType.get(resourceType) ?? [],
			pending: false,
			missing: !resourceType,
		}));

		if (unresolved.length > 0) {
			groups.push({ label: "", resourceIds: unresolved, pending: true, missing: false });
		}
		return groups;
	});

	// A role that only touches one kind of resource leaves nothing to decide —
	// preselect it so the picker below appears straight away. Only ever fills a
	// blank; it never overrides a type the user picked themselves.
	createEffect(() => {
		const types = applicableResourceTypes();
		if (types.length === 1 && !selectedResourceType()) {
			setSelectedResourceType(types[0]);
		}
	});

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
						<div class="flex-1 min-w-0 w-full">
							{/* A filter over the picker below, not a property of the grant —
							  switching it keeps everything already chosen, because the
							  scope is one flat id list with no type dimension. */}
							<InputDropdown
								onSelect={setSelectedResourceType}
								placeholder="Filter by type"
								value={selectedResourceType}
								options={applicableResourceTypes().map((resourceType) => ({
									label: parseCamelCase(resourceType),
									value: resourceType,
								}))}
							/>
						</div>
						<Show when={selectedResourceType()}>
							<div class="flex-1 min-w-0 w-full">
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
						{/* Grouped by type: the filter above shows one type at a time, so
						  without headings a chip row is an undifferentiated pile of names
						  and there's no sign the other types are still in there. */}
						<div class="flex flex-col gap-1.5">
							<For each={groupedSelection()}>
								{(group) => (
									<div class="flex flex-wrap items-center gap-1.5">
										<span class="text-grey text-xs shrink-0 w-full sm:w-auto sm:min-w-28">
											{group.label}
										</span>
										<For each={group.resourceIds}>
											{(resourceId) => (
												<Show
													when={!group.pending}
													fallback={
														<span class="inline-block h-5 w-24 bg-secondary border border-border-color rounded-xs animate-pulse" />
													}
												>
													<span
														title={group.missing ? resourceId : undefined}
														class={`inline-flex items-center gap-1.5 px-2 py-0.5 bg-secondary border rounded-xs text-xs ${
															group.missing
																? "border-warning-light text-warning"
																: "border-border-color text-white"
														}`}
													>
														{group.missing
															? "Deleted resource"
															: settled(resourcesInfoQuery)?.get(resourceId)?.name ||
																resourceId}
														<button
															type="button"
															aria-label="Remove resource from scope"
															onClick={() => toggleResource(resourceId)}
															class="text-grey hover:text-error transition-colors cursor-pointer"
														>
															<FiX size={11} />
														</button>
													</span>
												</Show>
											)}
										</For>
									</div>
								)}
							</For>
						</div>
					</Show>
				</Show>
			</div>
		</Suspense>
	);
};

export default ScopePicker;
