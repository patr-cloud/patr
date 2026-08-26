import { createMemo, For, Show } from "solid-js";
import { FiX } from "solid-icons/fi";
import InputDropdownCheckBox from "./input-dropdown-checkbox";
import ScopePicker from "./scope-picker";
import { Scope, groupScopes, scopeResources } from "~/utils/scope";
import { usePermissionsQuery } from "~/hooks/fetch";
import { parsePermissionName } from "~/utils/func";
import { WorkspacePermission } from "~/bindings/WorkspacePermission";

/**
 * One row in the editor: a permission held at one scope. The wire groups these
 * by permission id; the editor keeps them flat because that is how they are
 * added and removed.
 */
export type PermissionGrant = { permissionId: string; resourceId: string };

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

interface TokenPermissionEditorProps {
	workspaceId: string;
	grants: PermissionGrant[];
	/** Called with the complete next grant list on every edit. */
	onChange: (next: PermissionGrant[]) => void;
}

/**
 * Edits an API token's permission grants for one workspace: which permissions
 * it carries, and per grant, where it applies (see [ScopePicker]).
 *
 * Permissions rather than roles, because a role belongs to a workspace while a
 * token belongs to a user — and listing a workspace's roles is itself
 * permission-gated, so a member without it could not otherwise scope their own
 * token. The permission catalogue only needs workspace membership.
 */
const TokenPermissionEditor = (props: TokenPermissionEditorProps) => {
	const permissionsQuery = usePermissionsQuery(() => props.workspaceId);

	const permissionOptions = createMemo(() =>
		(permissionsQuery.data?.permissions ?? []).map((permission) => {
			const { resourceType, permission: action } = parsePermissionName(permission.name);
			return {
				label: resourceType ? `${resourceType} · ${action}` : permission.name,
				value: permission.id,
			};
		})
	);

	const permissionNameMap = createMemo(
		() => new Map(permissionOptions().map((option) => [option.value, option.label]))
	);
	const grantedPermissionIds = createMemo(() => props.grants.map((grant) => grant.permissionId));

	// Ceiling rows are flat — one per (permission, resource). The editor shows
	// one row per permission, so they are grouped here and expanded on save.
	const grouped = createMemo(() =>
		groupScopes(
			props.grants,
			(grant) => grant.permissionId,
			(grant) => grant.resourceId,
			props.workspaceId
		)
	);

	const togglePermission = (permissionId: string) => {
		const next = props.grants.some((grant) => grant.permissionId === permissionId)
			? props.grants.filter((grant) => grant.permissionId !== permissionId)
			: [...props.grants, { permissionId, resourceId: props.workspaceId }];
		props.onChange(next);
	};

	const updateScope = (permissionId: string, scope: Scope) => {
		props.onChange([
			...props.grants.filter((grant) => grant.permissionId !== permissionId),
			...scopeResources(scope, props.workspaceId).map((resourceId) => ({
				permissionId,
				resourceId,
			})),
		]);
	};

	return (
		<div class="flex flex-col gap-3">
			<Show
				when={grouped().length > 0}
				fallback={<p class="text-grey text-sm italic">No permissions selected.</p>}
			>
				<ul class="flex flex-col gap-2">
					<For each={grouped()}>
						{(entry) => (
							<li class="flex flex-col gap-2 p-3 border border-border-color rounded-xs">
								<div class="flex items-center justify-between gap-2">
									<span class="text-white text-sm font-medium truncate">
										{permissionNameMap().get(entry.subjectId) || entry.subjectId}
									</span>
									<button
										type="button"
										aria-label={`Remove ${permissionNameMap().get(entry.subjectId) || entry.subjectId}`}
										onClick={() => togglePermission(entry.subjectId)}
										class="text-grey hover:text-error transition-colors cursor-pointer"
									>
										<FiX size={14} />
									</button>
								</div>
								<ScopePicker
									workspaceId={props.workspaceId}
									permissionIds={[entry.subjectId]}
									scope={entry.scope}
									onChange={(scope) => updateScope(entry.subjectId, scope)}
								/>
							</li>
						)}
					</For>
				</ul>
			</Show>

			<div class="flex flex-col gap-2 p-3 border border-dashed border-border-color rounded-xs">
				<InputDropdownCheckBox
					placeholder="+ Add permission..."
					styleVariant="medium"
					options={permissionOptions()}
					checked={grantedPermissionIds()}
					onToggle={togglePermission}
				/>
			</div>
		</div>
	);
};

export default TokenPermissionEditor;
