import { Show } from "solid-js";
import { Checkbox, Radio, TokenPermissionEditor } from "~/components";
import type { PermissionGrant } from "~/components/token-permission-editor";
import { WithId, Workspace } from "~/bindings";

interface TokenGrantsItemProps {
	workspace: WithId<Workspace>;
	/** Whether the logged-in user owns this workspace — gates the super-admin option. */
	isSuperAdmin: boolean;
	enabled: boolean;
	/** The token holds super-admin on this workspace (rather than permission grants). */
	superAdmin: boolean;
	grants: PermissionGrant[];
	onToggle: (workspaceId: string, enabled: boolean) => void;
	onSuperAdminChange: (workspaceId: string, superAdmin: boolean) => void;
	onGrantsChange: (workspaceId: string, grants: PermissionGrant[]) => void;
}

/**
 * One workspace's slice of an API token's ceiling: either super-admin (owner
 * only), or a set of permission grants authored with [TokenPermissionEditor].
 * The grants are a ceiling, not a grant: at auth time they intersect with
 * whatever the token's owner can currently do.
 */
const TokenGrantsItem = (props: TokenGrantsItemProps) => {
	const permissionCount = () => new Set(props.grants.map((grant) => grant.permissionId)).size;

	return (
		<div class="w-full flex flex-col items-start gap-3 border border-border-color rounded-xs p-4">
			<div class="flex items-center justify-between gap-2 w-full">
				<Checkbox
					checked={props.enabled}
					onChange={() => props.onToggle(props.workspace.id, !props.enabled)}
					label={props.workspace.name}
				/>
				<Show when={props.enabled}>
					<span class="text-grey text-sm shrink-0">
						{props.superAdmin
							? "Super admin"
							: `${permissionCount()} permission${permissionCount() === 1 ? "" : "s"}`}
					</span>
				</Show>
			</div>

			<Show when={props.enabled}>
				<div class="flex flex-col gap-4 w-full">
					<Show when={props.isSuperAdmin}>
						<div class="flex flex-row items-center gap-6">
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
								label="Specific Permissions"
							/>
						</div>
					</Show>

					<Show when={!props.superAdmin}>
						<TokenPermissionEditor
							workspaceId={props.workspace.id}
							grants={props.grants}
							onChange={(next) => props.onGrantsChange(props.workspace.id, next)}
						/>
					</Show>
				</div>
			</Show>
		</div>
	);
};

export default TokenGrantsItem;
