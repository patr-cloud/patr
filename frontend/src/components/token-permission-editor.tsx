import { createMemo, createSignal, For, Show } from "solid-js";
import Input from "./input";
import TokenPermissionAddRow from "./token-permission-add-row";
import TokenPermissionRow from "./token-permission-row";
import { groupScopes } from "~/utils/scope";
import { usePermissionsQuery, useResourcesInfoQuery } from "~/hooks/fetch";
import { parseCamelCase, parsePermissionName } from "~/utils/func";
import { WorkspacePermission } from "~/bindings/WorkspacePermission";

/**
 * One row in the editor: a permission held at one scope. The wire groups these
 * by permission id; the editor keeps them flat because that is how they are
 * added and removed.
 */
export type PermissionGrant = { permissionId: string; resourceId: string };

/** A permission from the workspace's catalogue, split into what the editor shows. */
export type PermissionInfo = {
	id: string;
	/** The resource type the permission acts on, as named on the wire. */
	resourceType: string;
	/** The action, e.g. "View". */
	label: string;
	/** The resource type, e.g. "Deployment". */
	typeLabel: string;
};

/** Folds the editor's flat rows back into the wire shape for one workspace. */
export const toWorkspacePermission = (isSuperAdmin: boolean, grants: PermissionGrant[]): WorkspacePermission => {
	if (isSuperAdmin) {
		return { type: "superAdmin" };
	}

	const scopesByPermission: { [permissionId: string]: string[] } = {};
	for (const grant of grants) {
		(scopesByPermission[grant.permissionId] ??= []).push(grant.resourceId);
	}
	return { type: "member", ...scopesByPermission } as WorkspacePermission;
};

/** Expands one workspace's wire shape into the editor's flat rows. */
export const toPermissionGrants = (permission: WorkspacePermission): PermissionGrant[] => {
	if (permission.type === "superAdmin") {
		return [];
	}

	const { type: _type, ...scopesByPermission } = permission;
	return Object.entries(scopesByPermission).flatMap(([permissionId, scopes]) =>
		(scopes as string[]).map((resourceId) => ({ permissionId, resourceId }))
	);
};

/**
 * A stable string form of a token's ceiling, so two ceilings compare equal
 * regardless of key or scope order.
 */
export const canonicalPermissions = (permissions: { [workspaceId: string]: WorkspacePermission | undefined }) =>
	JSON.stringify(
		Object.keys(permissions)
			.sort()
			.map((workspaceId) => {
				const { type, ...scopes } = permissions[workspaceId] ?? { type: "member" };
				return [
					workspaceId,
					type,
					Object.keys(scopes)
						.sort()
						.map((permissionId) => [
							permissionId,
							[...((scopes as { [id: string]: string[] })[permissionId] ?? [])].sort(),
						]),
				];
			})
	);

/** Adds grants without dropping any already there; duplicates collapse. */
const mergeGrants = (existing: PermissionGrant[], added: PermissionGrant[]): PermissionGrant[] => {
	const key = (grant: PermissionGrant) => `${grant.permissionId}:${grant.resourceId}`;
	const seen = new Set(existing.map(key));
	return [
		...existing,
		...added.filter((grant) => {
			if (seen.has(key(grant))) return false;
			seen.add(key(grant));
			return true;
		}),
	];
};

interface TokenPermissionEditorProps {
	workspaceId: string;
	grants: PermissionGrant[];
	/** Called with the complete next grant list on every edit. */
	onChange: (next: PermissionGrant[]) => void;
}

/**
 * Edits an API token's permission grants for one workspace: an add row on top,
 * and a filterable table of what's granted below, editable one row at a time.
 *
 * Permissions rather than roles, because a role belongs to a workspace while a
 * token belongs to a user — and listing a workspace's roles is itself
 * permission-gated, so a member without it could not otherwise scope their own
 * token. The permission catalogue only needs workspace membership.
 */
const TokenPermissionEditor = (props: TokenPermissionEditorProps) => {
	const permissionsQuery = usePermissionsQuery(() => props.workspaceId);
	const [filter, setFilter] = createSignal("");
	const [editingId, setEditingId] = createSignal<string>();

	const catalogue = createMemo(
		() =>
			new Map(
				(permissionsQuery.data?.permissions ?? []).map((permission) => {
					const { resourceType, permission: action } = parsePermissionName(permission.name);
					return [
						permission.id,
						{
							id: permission.id,
							resourceType,
							label: action ? parseCamelCase(action) : permission.name,
							typeLabel: parseCamelCase(resourceType),
						} satisfies PermissionInfo,
					];
				})
			)
	);

	// Sorted by type so each type's permissions sit together under one heading.
	const options = createMemo(() =>
		[...catalogue().values()]
			.sort((a, b) => a.typeLabel.localeCompare(b.typeLabel) || a.label.localeCompare(b.label))
			.map((permission) => ({ label: permission.label, value: permission.id, group: permission.typeLabel }))
	);

	// Grants are flat — one per (permission, resource). The table shows one row
	// per permission, so they are grouped here and expanded on save.
	const rows = createMemo(() =>
		groupScopes(
			props.grants,
			(grant) => grant.permissionId,
			(grant) => grant.resourceId,
			props.workspaceId
		)
	);

	const filteredRows = createMemo(() => {
		const text = filter().trim().toLowerCase();
		if (!text) return rows();
		return rows().filter((row) => {
			const permission = catalogue().get(row.subjectId);
			return `${permission?.label ?? row.subjectId} ${permission?.typeLabel ?? ""}`.toLowerCase().includes(text);
		});
	});

	// Resolve scoped resource ids to names for the table, in one lookup.
	const resourceIds = createMemo(() => [
		...new Set(props.grants.map((grant) => grant.resourceId).filter((id) => id !== props.workspaceId)),
	]);
	const resourcesInfoQuery = useResourcesInfoQuery(
		() => resourceIds(),
		() => props.workspaceId
	);
	const resourceName = (resourceId: string) => {
		// Checked through status first so a cold lookup doesn't suspend the table.
		if (!resourcesInfoQuery.isSuccess || resourcesInfoQuery.isPlaceholderData) return "…";
		return resourcesInfoQuery.data?.get(resourceId)?.name || "Unknown resource";
	};

	const addGrants = (permissionIds: string[], scopeIds: string[]) => {
		props.onChange(
			mergeGrants(
				props.grants,
				permissionIds.flatMap((permissionId) => scopeIds.map((resourceId) => ({ permissionId, resourceId })))
			)
		);
	};

	// An explicit edit replaces that row's scope. If the row now names a
	// permission that is already granted, the two merge.
	const replaceRow = (oldPermissionId: string, permissionId: string, scopeIds: string[]) => {
		props.onChange(
			mergeGrants(
				props.grants.filter((grant) => grant.permissionId !== oldPermissionId),
				scopeIds.map((resourceId) => ({ permissionId, resourceId }))
			)
		);
		setEditingId();
	};

	const removeRow = (permissionId: string) => {
		props.onChange(props.grants.filter((grant) => grant.permissionId !== permissionId));
		if (editingId() === permissionId) setEditingId();
	};

	return (
		<div class="flex flex-col gap-3 w-full">
			<TokenPermissionAddRow
				workspaceId={props.workspaceId}
				catalogue={catalogue()}
				options={options()}
				onAdd={addGrants}
			/>

			<Show when={rows().length > 0} fallback={<p class="text-grey text-sm italic">No permissions added yet.</p>}>
				<div class="flex flex-col sm:flex-row sm:items-center justify-between gap-2">
					<span class="text-grey text-xs uppercase tracking-wide">{rows().length} assigned</span>
					<Input
						class="w-full sm:w-64"
						styleVariant="medium"
						placeholder="Filter assigned..."
						value={filter()}
						onInput={(e) => setFilter(e.currentTarget.value)}
					/>
				</div>
				<ul class="flex flex-col">
					<For
						each={filteredRows()}
						fallback={<li class="text-grey text-sm italic py-sm">No permissions match the filter.</li>}
					>
						{(row) => (
							<TokenPermissionRow
								workspaceId={props.workspaceId}
								permissionId={row.subjectId}
								scope={row.scope}
								catalogue={catalogue()}
								options={options()}
								resourceName={resourceName}
								editing={editingId() === row.subjectId}
								onEdit={() => setEditingId(row.subjectId)}
								onDone={(permissionId, scopeIds) => replaceRow(row.subjectId, permissionId, scopeIds)}
								onCancel={() => setEditingId()}
								onRemove={() => removeRow(row.subjectId)}
							/>
						)}
					</For>
				</ul>
			</Show>
		</div>
	);
};

export default TokenPermissionEditor;
