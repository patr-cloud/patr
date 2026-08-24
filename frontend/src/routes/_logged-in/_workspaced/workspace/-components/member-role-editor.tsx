import { createMemo, For, Show } from "solid-js";
import { FiX } from "solid-icons/fi";
import { InputDropdownCheckBox } from "~/components";
import { RoleGrant } from "~/bindings/RoleGrant";
import { WithId, WorkspaceRole } from "~/bindings";
import ScopePicker from "./scope-picker";

interface MemberRoleEditorProps {
	workspaceId: string;
	grants: RoleGrant[];
	/** Called with the complete next grant list on every edit. */
	onChange: (next: RoleGrant[]) => void;
	roles: WithId<WorkspaceRole>[];
}

/**
 * Edits an actor's role grants: which roles they hold, and per grant, where it
 * applies (see [ScopePicker]). Adding a role defaults to workspace scope —
 * the same reach the pre-scoped world gave — and narrowing it is opt-in.
 */
const MemberRoleEditor = (props: MemberRoleEditorProps) => {
	const roleNameMap = createMemo(() => new Map(props.roles.map((role) => [role.id, role.name])));
	const grantedRoleIds = createMemo(() => props.grants.map((grant) => grant.roleId));

	const toggleRole = (roleId: string) => {
		const next = props.grants.some((grant) => grant.roleId === roleId)
			? props.grants.filter((grant) => grant.roleId !== roleId)
			: [...props.grants, { roleId, scope: { scopeType: "workspace" as const } }];
		props.onChange(next);
	};

	const updateScope = (roleId: string, scope: RoleGrant["scope"]) => {
		props.onChange(props.grants.map((grant) => (grant.roleId === roleId ? { roleId, scope } : grant)));
	};

	return (
		<div class="flex flex-col gap-3">
			<Show when={props.grants.length > 0} fallback={<p class="text-grey text-sm italic">No roles assigned.</p>}>
				<ul class="flex flex-col gap-2">
					<For each={props.grants}>
						{(grant) => (
							<li class="flex flex-col gap-2 p-3 border border-border-color rounded-xs">
								<div class="flex items-center justify-between gap-2">
									<span class="text-white text-sm font-medium truncate">
										{roleNameMap().get(grant.roleId) || grant.roleId}
									</span>
									<button
										type="button"
										aria-label={`Remove ${roleNameMap().get(grant.roleId) || grant.roleId}`}
										onClick={() => toggleRole(grant.roleId)}
										class="text-grey hover:text-error transition-colors cursor-pointer"
									>
										<FiX size={14} />
									</button>
								</div>
								<ScopePicker
									workspaceId={props.workspaceId}
									roleId={grant.roleId}
									scope={grant.scope}
									onChange={(scope) => updateScope(grant.roleId, scope)}
								/>
							</li>
						)}
					</For>
				</ul>
			</Show>

			<div class="flex flex-col gap-2 p-3 border border-dashed border-border-color rounded-xs">
				<InputDropdownCheckBox
					placeholder="+ Add role..."
					styleVariant="medium"
					options={props.roles.map((role) => ({ label: role.name, value: role.id }))}
					checked={grantedRoleIds()}
					onToggle={toggleRole}
				/>
				<a
					href="/workspace/roles/new"
					target="_blank"
					rel="noopener noreferrer"
					class="text-primary text-xs hover:underline self-start"
				>
					or create a new role &rarr;
				</a>
			</div>
		</div>
	);
};

export default MemberRoleEditor;
