import { useParams } from "@tanstack/solid-router";
import { useQueryClient } from "@tanstack/solid-query";
import { createEffect, createMemo, createSignal, For, Show, Suspense } from "solid-js";
import { FiCheck, FiX } from "solid-icons/fi";
import { BindingRows, Button, ButtonVariant, useToast } from "~/components";
import type { Binding } from "~/components/binding-rows";
import { RoleBindingGrant } from "~/bindings/RoleBindingGrant";
import { UpdateUserRolesInWorkspaceRequest } from "~/bindings/UpdateUserRolesInWorkspaceRequest";
import { useMembersQuery, useRoleBindingsQuery } from "~/hooks/fetch";
import { useIsAllowed } from "~/hooks";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { groupScopes, scopeResources } from "~/utils/scope";
import { memberKeys, roleKeys } from "~/hooks/query-keys";
import { httpRequest } from "~/utils/http-request";

/** One holder of the role, read-only: who, and where the role applies to them. */
const HolderRow = (props: { name: string; scope: string }) => (
	<li class="flex items-center justify-between gap-3 px-3 py-2 border border-border-color rounded-xs">
		<span class="text-white text-sm truncate">{props.name}</span>
		<span class="text-grey text-xs shrink-0">{props.scope}</span>
	</li>
);

/**
 * The role's holders, as one row per binding: who holds it, and where it applies.
 * The same widget the members page uses, transposed — there the actor is fixed
 * and each row picks a role, here the role is fixed and each row picks a user.
 *
 * Members are edited through the member list, since saving one rewrites all of
 * their grants. Service accounts holding the role come from the role's bindings
 * and are read-only here.
 */
const UsersAssignedToRole = () => {
	const params = useParams({ from: "/_logged-in/_workspaced/workspace/roles/$roleId" });
	const [workspaceId] = useLastWorkspaceId();
	const queryClient = useQueryClient();
	const toast = useToast();
	const canModifyRoles = useIsAllowed("modifyRoles", "edit");

	// Unpaginated: the row editor needs every member's grants at once, both to
	// show who holds the role and to preserve their other roles on save.
	const membersQuery = useMembersQuery(
		() => undefined,
		() => undefined
	);

	const assignableMembers = createMemo(() => (membersQuery.data?.members ?? []).filter((member) => !member.isOwner));

	const bindingsQuery = useRoleBindingsQuery(() => params().roleId);

	/** One binding per service account holding this role. */
	const serviceAccountBindings = createMemo<Binding[]>(() =>
		groupScopes(
			(bindingsQuery.data ?? []).filter((binding) => binding.actor.type === "serviceAccount"),
			(binding) => binding.actor.id,
			(binding) => binding.resourceId,
			workspaceId() ?? ""
		)
	);

	const serviceAccountName = (id: string) => {
		const actor = (bindingsQuery.data ?? []).find((binding) => binding.actor.id === id)?.actor;
		return actor?.type === "serviceAccount" ? actor.name : id;
	};

	const userOptions = createMemo(() =>
		assignableMembers().map((member) => ({
			label: `${member.fullName} (${member.email})`,
			value: member.userId,
		}))
	);

	/** The saved state: one binding per member who holds this role. */
	const savedBindings = createMemo<Binding[]>(() =>
		assignableMembers().flatMap((member) => {
			// Flat grants: one row per resource, grouped back into one scope.
			const mine = member.grants.filter((g) => g.roleId === params().roleId);
			if (mine.length === 0) return [];
			const [grouped] = groupScopes(
				mine,
				() => member.userId,
				(grant) => grant.resourceId,
				workspaceId() ?? ""
			);
			return [grouped];
		})
	);

	const [isEditing, setIsEditing] = createSignal(false);
	const [draft, setDraft] = createSignal<Binding[]>([]);
	const [isSaving, setIsSaving] = createSignal(false);

	// Keep the read-only view in step with refetches while not editing.
	createEffect(() => {
		if (!isEditing()) setDraft(savedBindings());
	});

	const memberName = (userId: string) => {
		const member = assignableMembers().find((m) => m.userId === userId);
		return member ? `${member.fullName} (${member.email})` : userId;
	};

	const scopeLabel = (binding: Binding) => {
		if (binding.scope.scopeType !== "resources") return "Entire workspace";
		const count = binding.scope.resources.length;
		return `${count} resource${count === 1 ? "" : "s"}`;
	};

	const beginEditing = () => {
		setDraft(savedBindings());
		setIsEditing(true);
	};

	const cancelEditing = () => {
		setDraft(savedBindings());
		setIsEditing(false);
	};

	/**
	 * Writes one member's full grant list. There is no add/remove-single-role
	 * endpoint — `POST /rbac/user/:id` replaces the set — so every write carries
	 * the member's other roles through untouched.
	 */
	const writeMemberGrants = async (wsId: string, userId: string, grants: RoleBindingGrant[]) => {
		const body: UpdateUserRolesInWorkspaceRequest = { roles: grants };
		const response = await httpRequest(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/rbac/user/${userId}`,
			{ method: "POST", body: JSON.stringify(body) }
		);
		return response.ok;
	};

	const save = async () => {
		const wsId = workspaceId();
		if (!wsId) {
			toast("Workspace ID is missing", "error");
			return;
		}

		const roleId = params().roleId;
		const next = new Map(
			draft()
				.filter((b) => b.subjectId)
				.map((b) => [b.subjectId, b.scope])
		);
		const before = new Map(savedBindings().map((b) => [b.subjectId, b.scope]));

		// Only touch members whose binding for this role actually changed.
		const touched = new Set([...next.keys(), ...before.keys()]).values();
		const writes: { userId: string; grants: RoleBindingGrant[] }[] = [];
		for (const userId of touched) {
			const nextScope = next.get(userId);
			const beforeScope = before.get(userId);
			if (JSON.stringify(nextScope ?? null) === JSON.stringify(beforeScope ?? null)) continue;

			const member = assignableMembers().find((m) => m.userId === userId);
			if (!member) continue;
			const others = member.grants.filter((g) => g.roleId !== roleId);
			const mine = nextScope ? scopeResources(nextScope, wsId).map((resourceId) => ({ roleId, resourceId })) : [];
			writes.push({ userId, grants: [...others, ...mine] });
		}

		if (writes.length === 0) {
			setIsEditing(false);
			return;
		}

		setIsSaving(true);
		const results = await Promise.all(writes.map((w) => writeMemberGrants(wsId, w.userId, w.grants)));
		setIsSaving(false);

		const failed = results.filter((ok) => !ok).length;
		if (failed > 0) {
			toast(`Failed to update ${failed} of ${writes.length} members`, "error");
		} else {
			toast(writes.length === 1 ? "Member updated" : `${writes.length} members updated`, "success");
		}

		queryClient.invalidateQueries({ queryKey: memberKeys.all(wsId) });
		queryClient.invalidateQueries({ queryKey: roleKeys.bindings(wsId, roleId) });
		setIsEditing(false);
	};

	return (
		<div class="flex flex-col gap-4">
			<div class="flex items-center justify-between">
				<div class="flex flex-col gap-1">
					<h3 class="text-lg text-white">Users with this role</h3>
					<p class="text-grey text-xs">
						Each row is one binding — the user, and where this role applies to them.
					</p>
				</div>
				<Show when={canModifyRoles()}>
					<Show
						when={isEditing()}
						fallback={
							<Button variant={ButtonVariant.Outlined} onClick={beginEditing}>
								Edit users
							</Button>
						}
					>
						<div class="flex items-center gap-2">
							<Button
								variant={ButtonVariant.Contained}
								class="flex items-center gap-2"
								loading={isSaving()}
								onClick={() => save().catch(() => {})}
							>
								<FiCheck size={14} />
								Save
							</Button>
							<Button
								variant={ButtonVariant.Outlined}
								class="flex items-center gap-2"
								onClick={cancelEditing}
							>
								<FiX size={14} />
								Cancel
							</Button>
						</div>
					</Show>
				</Show>
			</div>

			<Suspense
				fallback={<div class="flex items-center justify-center py-8 text-grey text-sm">Loading users...</div>}
			>
				<Show
					when={isEditing()}
					fallback={
						<Show
							when={savedBindings().length > 0}
							fallback={
								<div class="text-grey text-center py-8">No users have been assigned this role yet</div>
							}
						>
							<ul class="flex flex-col gap-2">
								<For each={savedBindings()}>
									{(binding) => (
										<HolderRow name={memberName(binding.subjectId)} scope={scopeLabel(binding)} />
									)}
								</For>
							</ul>
						</Show>
					}
				>
					<BindingRows
						workspaceId={workspaceId()!}
						bindings={draft()}
						onChange={setDraft}
						subjectOptions={userOptions()}
						subjectPlaceholder="Select a user"
						// The role is fixed here, so every row's scope is bounded by
						// this role's permissions regardless of who holds it.
						scopeRoleId={() => params().roleId}
						addLabel="Add user"
						emptyText="No users have this role yet."
					/>
				</Show>
			</Suspense>

			<Show when={serviceAccountBindings().length > 0}>
				<div class="flex flex-col gap-2">
					<div class="flex flex-col gap-1">
						<h3 class="text-lg text-white">Service accounts with this role</h3>
						<p class="text-grey text-xs">
							Read-only here: a service account's roles are set on the account.
						</p>
					</div>
					<ul class="flex flex-col gap-2">
						<For each={serviceAccountBindings()}>
							{(binding) => (
								<HolderRow name={serviceAccountName(binding.subjectId)} scope={scopeLabel(binding)} />
							)}
						</For>
					</ul>
				</div>
			</Show>
		</div>
	);
};

export default UsersAssignedToRole;
