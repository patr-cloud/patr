import { useQueryClient } from "@tanstack/solid-query";
import { FiX } from "solid-icons/fi";
import { createSignal, For, Show } from "solid-js";
import { Initials, LoadingSpinner, useToast } from "~/components";
import { UpdateUserRolesInWorkspaceRequest } from "~/bindings/UpdateUserRolesInWorkspaceRequest";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { useMembersQuery, useRoleUsersQuery } from "~/hooks/fetch";
import { memberKeys, roleKeys } from "~/hooks/query-keys";
import { httpRequest } from "~/utils/http-request";
import { Color } from "~/utils/color";

interface RoleUsersChipsProps {
	roleId: string;
}

const RoleUsersChips = (props: RoleUsersChipsProps) => {
	const [workspaceId] = useLastWorkspaceId();
	const queryClient = useQueryClient();
	const toast = useToast();

	const usersQuery = useRoleUsersQuery(() => props.roleId);
	// Unpaginated fetch so we can read the target user's current role set before
	// dropping this role from it (the backend has no "remove single role"
	// endpoint — only a full replace on POST /rbac/user/:userId).
	// TODO: replace with a dedicated single-role add/remove endpoint once the
	// backend exposes one — pulling every member + N user details per expansion
	// doesn't scale.
	const membersQuery = useMembersQuery(
		() => undefined,
		() => undefined
	);

	const [isMutating, setIsMutating] = createSignal(false);

	const getCurrentRoleIds = (userId: string): string[] => {
		const member = membersQuery.data?.members.find((m) => m.userId === userId);
		return member?.roleIds ?? [];
	};

	const updateUserRoles = async (userId: string, roleIds: string[]): Promise<boolean> => {
		const wsId = workspaceId();
		if (!wsId) {
			toast("Workspace ID is missing", "error");
			return false;
		}
		const body: UpdateUserRolesInWorkspaceRequest = {
			roles: roleIds.map((roleId) => ({ roleId, scope: { scopeType: "workspace" as const } })),
		};
		const response = await httpRequest(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/rbac/user/${userId}`,
			{ method: "POST", body: JSON.stringify(body) }
		);
		if (!response.ok) {
			toast(response.data.error || "Failed to update user roles", "error");
			return false;
		}
		queryClient.invalidateQueries({ queryKey: roleKeys.users(wsId, props.roleId) });
		queryClient.invalidateQueries({ queryKey: memberKeys.all(wsId) });
		return true;
	};

	const removeUser = async (userId: string, username: string) => {
		if (isMutating() || !membersQuery.data) {
			if (!membersQuery.data) toast("Members not loaded yet, try again", "error");
			return;
		}
		setIsMutating(true);
		const next = getCurrentRoleIds(userId).filter((id) => id !== props.roleId);
		const ok = await updateUserRoles(userId, next);
		setIsMutating(false);
		if (ok) toast(`Removed ${username} from role`, "success");
	};

	const canMutate = () => !isMutating() && !!membersQuery.data;

	return (
		<div class="flex flex-col gap-3">
			{/* Chips */}
			<Show
				when={!usersQuery.isPending}
				fallback={
					<div class="flex items-center gap-2 text-grey text-xs">
						<LoadingSpinner size={14} />
						<span>Loading users...</span>
					</div>
				}
			>
				<Show
					when={(usersQuery.data ?? []).length > 0}
					fallback={<span class="text-grey text-xs italic">No users assigned</span>}
				>
					<div class="flex flex-wrap gap-2">
						<For each={usersQuery.data ?? []}>
							{(user) => (
								<div class="flex items-center gap-1.5 bg-secondary rounded-full pl-1 pr-1 py-0.5 text-xs text-white">
									<Initials
										bgColor={Color.Secondary}
										firstName={user.firstName}
										lastName={user.lastName}
										size="xs"
									/>
									<span class="font-mono truncate max-w-40">{user.username}</span>
									<button
										type="button"
										aria-label={`Remove ${user.username} from role`}
										onClick={() => removeUser(user.id, user.username).catch(() => {})}
										disabled={!canMutate()}
										class="text-grey hover:text-error transition-colors p-0.5 rounded-full cursor-pointer disabled:cursor-not-allowed disabled:opacity-50"
									>
										<FiX size={12} />
									</button>
								</div>
							)}
						</For>
					</div>
				</Show>
			</Show>
		</div>
	);
};

export default RoleUsersChips;
