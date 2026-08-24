import { Show } from "solid-js";
import { Checkbox, MemberRoleEditor, Radio } from "~/components";
import { RoleGrant } from "~/bindings/RoleGrant";
import { WithId, Workspace } from "~/bindings";
import { useAllRolesQuery } from "~/hooks/fetch";

interface TokenGrantsItemProps {
	workspace: WithId<Workspace>;
	/** Whether the logged-in user owns this workspace — gates the super-admin option. */
	isSuperAdmin: boolean;
	enabled: boolean;
	/** The token holds super-admin on this workspace (rather than role grants). */
	superAdmin: boolean;
	grants: RoleGrant[];
	onToggle: (workspaceId: string, enabled: boolean) => void;
	onSuperAdminChange: (workspaceId: string, superAdmin: boolean) => void;
	onGrantsChange: (workspaceId: string, grants: RoleGrant[]) => void;
}

/**
 * One workspace's slice of an API token's ceiling: either super-admin (owner
 * only), or a set of role grants authored with [MemberRoleEditor] — the same
 * roles-and-scopes vocabulary used for members. The grants are a ceiling, not
 * a grant: at auth time they intersect with whatever the token's owner can
 * currently do.
 */
const TokenGrantsItem = (props: TokenGrantsItemProps) => {
	// The single-page checkbox list needs every role at once; 100 is the
	// largest allowed page size (matches the members page's role picker).
	const rolesQuery = useAllRolesQuery(
		() => undefined,
		() => "100",
		() => props.workspace.id
	);

	return (
		<div class="w-full flex flex-col items-start gap-2 border border-border-color rounded-xs p-4">
			<Checkbox
				checked={props.enabled}
				onChange={() => props.onToggle(props.workspace.id, !props.enabled)}
				label={props.workspace.name}
			/>

			<Show when={props.enabled}>
				<div class="flex flex-col gap-4 w-full">
					<Show when={props.isSuperAdmin}>
						<div class="flex flex-row items-center gap-6 mt-2">
							<Radio
								name={`grant-mode-${props.workspace.id}`}
								checked={props.superAdmin}
								onChange={() => props.onSuperAdminChange(props.workspace.id, true)}
								label="Super Admin"
							/>
							<Radio
								name={`grant-mode-${props.workspace.id}`}
								checked={!props.superAdmin}
								onChange={() => props.onSuperAdminChange(props.workspace.id, false)}
								label="Specific Roles"
							/>
						</div>
					</Show>

					<Show when={!props.superAdmin}>
						<MemberRoleEditor
							workspaceId={props.workspace.id}
							grants={props.grants}
							onChange={(next) => props.onGrantsChange(props.workspace.id, next)}
							roles={rolesQuery.data?.roles ?? []}
							hideCreateRoleLink
						/>
					</Show>
				</div>
			</Show>
		</div>
	);
};

export default TokenGrantsItem;
