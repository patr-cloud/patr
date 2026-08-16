import { FiChevronDown, FiChevronUp, FiTrash2, FiX } from "solid-icons/fi";
import { createSignal, For, Show } from "solid-js";
import { Button, ButtonVariant, LoadingSpinner } from "~/components";
import { ResourceInfo } from "~/bindings/ResourceInfo";
import { ResourcePermissionType } from "~/bindings/ResourcePermissionType";
import { parseCamelCase } from "~/utils/func";

export interface PermissionEntry {
	permissionId: string;
	resourceType: string;
	action: string;
	permissionType: string;
	resources: string[];
}

/**
 * Drops a single resource from a permission's resource list.
 *
 * An `include` permission that has been emptied grants nothing at all, so the
 * permission is dropped entirely rather than left behind as a dead entry — an
 * empty resource list would otherwise render as "All resources", the exact
 * opposite of what it means. An emptied `exclude` genuinely does mean "all
 * resources", so it is kept.
 */
export const removeResourceFromPermissions = (
	data: { [key: string]: ResourcePermissionType },
	permissionId: string,
	resourceId: string
): { [key: string]: ResourcePermissionType } => {
	const existing = data[permissionId];
	if (!existing) return data;

	const resources = existing.resources.filter((id) => id !== resourceId);

	if (existing.permissionType === "include" && resources.length === 0) {
		const next = { ...data };
		delete next[permissionId];
		return next;
	}

	return { ...data, [permissionId]: { ...existing, resources } };
};

interface PermissionRowProps {
	perm: PermissionEntry;
	/** Resolved metadata for every resource ID in the table, keyed by ID. */
	resourceInfo: Map<string, ResourceInfo> | undefined;
	isLoadingResources: boolean;
	onRemove: () => void;
	onRemoveResource: (resourceId: string) => void;
}

/**
 * A single row of the role permissions table, shared by the create-role and
 * edit-role pages.
 *
 * A permission scoped to specific resources stores only their IDs, so the row
 * expands (in the same manner as the "See users" row on the roles list) to show
 * what those resources actually are. Rows that apply to every resource have
 * nothing to expand.
 */
const PermissionRow = (props: PermissionRowProps) => {
	const [expanded, setExpanded] = createSignal(false);

	const count = () => props.perm.resources.length;
	// "include" grants access to only the listed resources; "exclude" grants
	// access to everything but them. Losing that distinction would misrepresent
	// the permission entirely, so it stays in the summary.
	const prefix = () => (props.perm.permissionType === "include" ? "Only " : "All except ");

	return (
		<tr
			role="row"
			class="flex flex-col w-full border border-border-color bg-secondary-light last-of-type:rounded-b-xs hover:bg-secondary-medium"
		>
			{/* Inner flex ratios below must stay in sync with the parent Table's column_grids. */}
			<td role="cell" class="flex items-center justify-center min-h-10 w-full px-md md:px-xl">
				<div class="flex-4 flex items-center justify-start min-w-0">
					<span class="truncate">{parseCamelCase(props.perm.resourceType)}</span>
				</div>
				<div class="flex-3 flex items-center justify-start min-w-0">
					{/* Workspace-level permissions (viewRoles, modifyRoles, editWorkspace) are
					    modelled as resource types with no actions, so this cell is empty for them. */}
					<Show when={props.perm.action} fallback={<span class="text-gray-400">—</span>}>
						<span class="truncate">{parseCamelCase(props.perm.action)}</span>
					</Show>
				</div>
				<div class="flex-4 flex items-center justify-start min-w-0">
					<Show when={count() > 0} fallback={<span class="text-gray-400">All resources</span>}>
						<Button
							variant={ButtonVariant.Plain}
							aria-label={expanded() ? "Hide resources" : "Show resources"}
							aria-expanded={expanded()}
							onClick={() => setExpanded(!expanded())}
							class="flex items-center gap-1 cursor-pointer"
						>
							<span class="text-sm">
								{prefix()}
								{count()} resource{count() !== 1 ? "s" : ""}
							</span>
							{expanded() ? <FiChevronUp size={14} /> : <FiChevronDown size={14} />}
						</Button>
					</Show>
				</div>
				<div class="flex-1 flex items-center justify-center min-w-0">
					<button
						type="button"
						aria-label="Remove permission"
						class="text-error hover:bg-white/10 p-1 rounded transition-colors cursor-pointer"
						onClick={() => props.onRemove()}
					>
						<FiTrash2 size={16} />
					</button>
				</div>
			</td>
			<Show when={expanded()}>
				<td role="cell" class="w-full px-md md:px-xl py-sm border-t border-border-color/40">
					<Show
						when={!props.isLoadingResources}
						fallback={
							<div class="flex items-center gap-2 text-grey text-xs">
								<LoadingSpinner size={14} />
								<span>Loading resources...</span>
							</div>
						}
					>
						<div class="flex flex-wrap gap-2">
							<For each={props.perm.resources}>
								{(resourceId) => {
									const info = () => props.resourceInfo?.get(resourceId);
									return (
										<div class="flex items-center gap-1.5 bg-secondary rounded-full px-2 py-0.5 text-xs max-w-full">
											<Show
												when={info()?.name}
												fallback={
													// The resource was deleted, or its type carries no
													// name. Show the raw ID so the entry is still
													// identifiable rather than silently missing.
													<span
														class="font-mono truncate text-grey italic"
														title={resourceId}
													>
														{resourceId}
													</span>
												}
											>
												<span class="text-grey shrink-0">
													{parseCamelCase(info()!.resourceType)}:
												</span>
												<span class="truncate text-white" title={info()!.name!}>
													{info()!.name}
												</span>
											</Show>
											<button
												type="button"
												aria-label={`Remove ${info()?.name ?? resourceId}`}
												onClick={() => props.onRemoveResource(resourceId)}
												class="text-grey hover:text-error transition-colors p-0.5 rounded-full cursor-pointer shrink-0"
											>
												<FiX size={12} />
											</button>
										</div>
									);
								}}
							</For>
						</div>
					</Show>
				</td>
			</Show>
		</tr>
	);
};

export default PermissionRow;
