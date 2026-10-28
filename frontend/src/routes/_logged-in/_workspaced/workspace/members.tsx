import { createFileRoute } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import { createEffect, createMemo, createSignal, For, Show, Suspense } from "solid-js";
import {
	Button,
	ButtonVariant,
	BindingRows,
	Link,
	PageContainer,
	PageContainerBody,
	Pagination,
	UnsavedChangesGuard,
	useToast,
	Initials,
} from "~/components";
import { createBindingsDraft } from "~/components/binding-rows";
import { FiCopy, FiMail, FiTrash } from "solid-icons/fi";
import { useLocation, useNavigate } from "@tanstack/solid-router";
import {
	createAuthenticatedAction,
	createPaginationState,
	useIsAllowed,
	recoverFromOutOfBounds,
	paginationSearch,
} from "~/hooks";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { UpdateUserRolesInWorkspaceRequest } from "~/bindings/UpdateUserRolesInWorkspaceRequest";
import { RemoveUserFromWorkspaceResponse } from "~/bindings/RemoveUserFromWorkspaceResponse";
import { ResendWorkspaceInviteResponse } from "~/bindings/ResendWorkspaceInviteResponse";
import { UpdateWorkspaceInviteRolesRequest } from "~/bindings/UpdateWorkspaceInviteRolesRequest";
import { RoleBindingGrant } from "~/bindings/RoleBindingGrant";
import { httpRequest } from "~/utils/http-request";
import { bindingsToGrants, grantsToBindings } from "~/utils/scope";
import WorkspaceHeader from "./-components/workspace-header";
import { useWorkspaceInfoQuery, useRolePicker, useMembersQuery, useInvitesQuery } from "~/hooks/fetch";
import type { WorkspaceMember } from "~/hooks/fetch/members";
import { useQueryClient } from "@tanstack/solid-query";
import { memberKeys, inviteKeys } from "~/hooks/query-keys";

/**
 * A row in the people list. Invites and members are the same thing at different
 * stages — both are an email or a name holding a set of role grants — so they
 * share one list and one editor, with invites pinned first.
 */
type PersonRow = {
	/** Unique across both kinds, so selection survives a list refetch. */
	key: string;
	kind: "invite" | "member";
	/** Invite id, or user id. */
	id: string;
	title: string;
	subtitle: string;
	grants: RoleBindingGrant[];
	firstName: string;
	lastName: string;
	isOwner: boolean;
	expired: boolean;
};

const ManageWorkspace = () => {
	const [workspaceId] = useLastWorkspaceId();
	const toast = useToast();
	const navigate = useNavigate();
	const search = Route.useSearch();
	const pagination = createPaginationState({
		search: () => search(),
		navigate,
	});
	const queryClient = useQueryClient();

	const workspaceInfoQuery = useWorkspaceInfoQuery();
	const membersQuery = useMembersQuery(
		() => search().page?.toString(),
		() => search().count?.toString()
	);
	const invitesQuery = useInvitesQuery();
	const canModifyMembers = useIsAllowed("modifyRoles", "edit");

	// An expired invite is still listed for a while so it can be resent, but its
	// link no longer works — say so rather than showing it as merely pending.
	const isExpired = (expiry: Date) => new Date(expiry).getTime() <= Date.now();

	const isFirstPage = createMemo(() => !search().page);

	const toMemberRow = (member: WorkspaceMember): PersonRow => ({
		key: `member:${member.userId}`,
		kind: "member",
		id: member.userId,
		title: member.fullName,
		subtitle: member.email,
		grants: member.grants,
		firstName: member.firstName,
		lastName: member.lastName,
		isOwner: member.isOwner,
		expired: false,
	});

	/**
	 * Pending invites are pinned to the first page. They're fetched
	 * unpaginated, so repeating them on page 2 would be a lie about where they
	 * sit. The owner isn't pinned: the list endpoint returns them as a member
	 * like any other, flagged `isOwner`, so they're already inside the
	 * paginated set and counted there.
	 */
	const pinnedRows = createMemo<PersonRow[]>(() => {
		if (!isFirstPage()) return [];

		return (invitesQuery.data ?? []).map((invite): PersonRow => ({
			key: `invite:${invite.id}`,
			kind: "invite",
			id: invite.id,
			title: invite.email,
			subtitle: isExpired(invite.expiry) ? "Invite expired" : "Invited",
			grants: invite.roles,
			firstName: invite.email.slice(0, 1).toUpperCase(),
			lastName: "",
			isOwner: false,
			expired: isExpired(invite.expiry),
		}));
	});

	/** How many rows sit outside the paginated set, for the range label. */
	const pinnedCount = createMemo(() => (isFirstPage() ? (invitesQuery.data?.length ?? 0) : 0));

	// Owner, then anyone still pending, then the rest — the list reads top-down
	// from "runs this workspace" to "hasn't accepted yet" to "already here".
	const people = createMemo<PersonRow[]>(() => {
		const listed = membersQuery.data?.members ?? [];
		return [
			...listed.filter((member) => member.isOwner).map(toMemberRow),
			...pinnedRows(),
			...listed.filter((member) => !member.isOwner).map(toMemberRow),
		];
	});

	createEffect(() => {
		const totalCount = membersQuery.data?.totalCount;
		if (totalCount !== undefined) {
			pagination.setTotalCount(totalCount);
		}
	});

	recoverFromOutOfBounds(
		() => membersQuery.isError,
		() => membersQuery.error?.message,
		pagination
	);

	const refetchMembers = () => {
		const wsId = workspaceId();
		if (wsId) {
			queryClient.invalidateQueries({ queryKey: memberKeys.all(wsId) });
		}
	};

	const refetchInvites = () => {
		const wsId = workspaceId();
		if (wsId) {
			queryClient.invalidateQueries({ queryKey: inviteKeys.all(wsId) });
		}
	};

	const [selectedKey, setSelectedKey] = createSignal<string | null>(null);
	const [pendingDelete, setPendingDelete] = createSignal(false);
	// Accept links are only returned when an invite is created or resent, so the
	// "copy link" button only appears for invites touched in this session.
	const [inviteLinks, setInviteLinks] = createSignal<Record<string, string>>({});

	// An invite created on the invite page hands its accept link back through
	// history state, since the token is never retrievable again.
	const location = useLocation();
	createEffect(() => {
		const handoff = (location().state as { newInvite?: { id: string; acceptUrl: string } })?.newInvite;
		if (!handoff) return;
		setInviteLinks((prev) => ({ ...prev, [handoff.id]: handoff.acceptUrl }));
	});

	createEffect(() => {
		const rows = people();
		if (rows.length === 0) return;
		if (selectedKey() === null || !rows.some((row) => row.key === selectedKey())) {
			setSelectedKey(rows[0].key);
		}
	});

	const selected = createMemo(() => people().find((row) => row.key === selectedKey()) ?? null);

	// The selected person's access, edited in place and saved with one button.
	// The table shows one row per role, so flat grants are grouped.
	const access = createBindingsDraft(() => {
		const row = selected();
		return row ? grantsToBindings(row.grants, workspaceId() ?? "") : [];
	});
	const rolePicker = useRolePicker(() => access.draft().map((binding) => binding.subjectId));

	const selectRow = (key: string) => {
		if (key === selectedKey()) return;
		// The draft belongs to the selected person, so leaving it would lose it.
		if (access.isDirty()) {
			toast("Save or discard your changes first", "warn");
			return;
		}
		setSelectedKey(key);
		setPendingDelete(false);
	};

	const { execute: saveBindings, isLoading: isSaving } = createAuthenticatedAction(async ({ workspaceId }) => {
		const row = selected();
		if (!row) return;
		const grants = bindingsToGrants(access.draft(), workspaceId);

		if (row.kind === "member") {
			const body: UpdateUserRolesInWorkspaceRequest = { roles: grants };
			const response = await httpRequest(
				`${import.meta.env.VITE_BASE_URL}/api/workspace/${workspaceId}/rbac/user/${row.id}`,
				{ method: "POST", body: JSON.stringify(body) }
			);
			if (!response.ok) {
				toast(response.data.error || "Failed to update roles", "error");
				return;
			}
			toast("Roles updated successfully", "success");
			refetchMembers();
		} else {
			const body: UpdateWorkspaceInviteRolesRequest = { roles: grants };
			const response = await httpRequest(
				`${import.meta.env.VITE_BASE_URL}/api/workspace/${workspaceId}/rbac/user/invite/${row.id}`,
				{ method: "PATCH", body: JSON.stringify(body) }
			);
			if (!response.ok) {
				toast(response.data.error || "Failed to update invite", "error");
				return;
			}
			toast("Invite updated", "success");
			refetchInvites();
		}

		access.markSaved();
	});

	const { execute: removePerson, isLoading: isRemoving } = createAuthenticatedAction(async ({ workspaceId }) => {
		const row = selected();
		if (!row) return;

		if (row.kind === "member") {
			const response = await httpRequest<RemoveUserFromWorkspaceResponse>(
				`${import.meta.env.VITE_BASE_URL}/api/workspace/${workspaceId}/rbac/user/${row.id}`,
				{ method: "DELETE" }
			);
			if (!response.ok) {
				toast(response.data.error || "Failed to remove member", "error");
				return;
			}
			toast("User removed successfully", "success");
			refetchMembers();
		} else {
			const response = await httpRequest(
				`${import.meta.env.VITE_BASE_URL}/api/workspace/${workspaceId}/rbac/user/invite/${row.id}`,
				{ method: "DELETE" }
			);
			if (!response.ok) {
				toast(response.data.error || "Failed to revoke invite", "error");
				return;
			}
			toast("Invite revoked", "success");
			refetchInvites();
		}

		setPendingDelete(false);
		// The person is gone, so is any draft of their access.
		access.discard();
		setSelectedKey(null);
	});

	const resendInvite = async (inviteId: string) => {
		const wsId = workspaceId();
		if (!wsId) return;

		const response = await httpRequest<ResendWorkspaceInviteResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/rbac/user/invite/${inviteId}/resend`,
			{ method: "POST" }
		);

		if (!response.ok) {
			toast("Failed to resend invite", "error");
			return;
		}

		setInviteLinks((prev) => ({ ...prev, [inviteId]: response.data.acceptUrl }));
		toast("Invite resent", "success");
		refetchInvites();
	};

	const copyInviteLink = async (inviteId: string) => {
		const link = inviteLinks()[inviteId];
		if (!link) return;
		try {
			await navigator.clipboard.writeText(link);
			toast("Invite link copied", "success");
		} catch {
			toast("Couldn't copy the link", "error");
		}
	};

	/** Whether the selected person's access can be changed here at all. */
	const canEditAccess = () => canModifyMembers() && !selected()?.isOwner;

	return (
		<>
			<Title>Workspace Members | Patr</Title>
			<UnsavedChangesGuard
				when={access.isDirty}
				message="You have unsaved changes to this person's access. If you leave now, they'll be lost."
			/>
			<PageContainer fillViewport>
				<WorkspaceHeader
					workspaceName={workspaceInfoQuery.data?.name}
					activeTab="members"
					actions={() => (
						<Show when={canModifyMembers()}>
							<Link href="/workspace/members/invite" buttonVariant={ButtonVariant.Plain} external={false}>
								Invite Member
							</Link>
						</Show>
					)}
				/>
				<PageContainerBody class="flex flex-col justify-between gap-4">
					<div class="flex flex-col gap-6 flex-1 min-h-0">
						<div class="flex flex-1 min-h-0 flex-col lg:flex-row gap-6 lg:items-stretch">
							{/* Narrow rail: the editor beside it is what needs the room. It
							  scrolls on its own too — a long member list would otherwise be
							  clipped now that the page doesn't scroll. */}
							<div class="flex-1 w-full min-h-0 bg-secondary-light rounded-xs overflow-y-auto">
								<Suspense
									fallback={
										<div class="flex items-center justify-center py-16 text-grey">
											<span class="text-sm">Loading members...</span>
										</div>
									}
								>
									<Show
										when={people().length > 0}
										fallback={
											<div class="flex items-center justify-center py-16 text-grey">
												<span class="text-sm">No members found.</span>
											</div>
										}
									>
										<ul class="flex flex-col gap-2 p-2">
											<For each={people()}>
												{(row) => {
													const isSelected = () => selectedKey() === row.key;
													return (
														<li
															role="button"
															tabIndex={0}
															onClick={() => selectRow(row.key)}
															onKeyDown={(e) => {
																if (e.key === "Enter" || e.key === " ") {
																	e.preventDefault();
																	selectRow(row.key);
																}
															}}
															class={`relative flex items-center gap-3 px-lg py-4 cursor-pointer rounded-xs border border-border-color border-l-2 transition-colors hover:bg-secondary-medium ${
																isSelected()
																	? "border-l-primary bg-secondary-medium"
																	: "border-l-border-color"
															}`}
														>
															<Initials
																size="sm"
																firstName={row.firstName}
																lastName={row.lastName}
															/>
															{/* Badges sit under the name rather than beside it: the rail is
																  narrow now, and a badge on the same line squeezed every name down
																  to an ellipsis. The chevron went for the same reason — the left
																  border already marks the selection. */}
															<div class="flex flex-col min-w-0 flex-1 gap-0.5">
																<span class="text-white font-medium truncate">
																	{row.title}
																</span>
																<span class="flex items-center gap-2 min-w-0">
																	<span
																		class={`text-xs truncate ${row.expired ? "text-warning" : "text-grey"}`}
																	>
																		{row.subtitle}
																	</span>
																	<Show when={row.kind === "invite"}>
																		<span class="shrink-0 text-xs text-grey border border-border-color rounded-xs px-1.5">
																			Pending
																		</span>
																	</Show>
																	<Show when={row.isOwner}>
																		<span class="shrink-0 text-xs text-primary border border-primary rounded-xs px-1.5">
																			Owner
																		</span>
																	</Show>
																</span>
															</div>
														</li>
													);
												}}
											</For>
										</ul>
									</Show>
								</Suspense>
							</div>

							{/* Wide side: a binding row carries four controls, so it takes
							  the lion's share. Scrolls independently — the bindings run long
							  before the member list does. */}
							<div class="flex-3 w-full min-h-0 overflow-y-auto">
								<Show
									when={selected()}
									fallback={
										<div class="p-lg text-grey text-sm flex items-center justify-center min-h-50">
											Select someone to see their access.
										</div>
									}
								>
									{(row) => (
										<div class="p-lg flex flex-col gap-5">
											<div class="flex items-start justify-between gap-3">
												<Initials
													size="lg"
													styleVariant="medium"
													firstName={row().firstName}
													lastName={row().lastName}
												/>
												<Show when={!pendingDelete() && canEditAccess()}>
													<div class="flex items-center gap-2">
														<Show when={row().kind === "invite"}>
															<Show when={inviteLinks()[row().id]}>
																<Button
																	variant={ButtonVariant.Outlined}
																	class="flex items-center gap-2"
																	onClick={() => copyInviteLink(row().id)}
																>
																	<FiCopy size={14} />
																	Copy link
																</Button>
															</Show>
															<Button
																variant={ButtonVariant.Outlined}
																class="flex items-center gap-2"
																onClick={() => resendInvite(row().id)}
															>
																<FiMail size={14} />
																Resend
															</Button>
														</Show>
														<button
															aria-label={
																row().kind === "invite"
																	? "Revoke invite"
																	: "Remove member"
															}
															onClick={() => setPendingDelete(true)}
															class="text-error border border-border-color hover:bg-white/10 p-2 rounded-xs transition-colors cursor-pointer"
														>
															<FiTrash size={16} />
														</button>
													</div>
												</Show>
											</div>

											<div class="flex flex-col gap-1">
												<span class="text-white text-xl font-medium">{row().title}</span>
												<span class={`text-sm ${row().expired ? "text-warning" : "text-grey"}`}>
													{row().subtitle}
												</span>
											</div>

											<div class="flex flex-col gap-3">
												<div class="flex items-center justify-between">
													<h3 class="text-white text-sm font-medium">Access</h3>
													<span class="text-grey text-xs">{access.draft().length}</span>
												</div>
												<BindingRows
													workspaceId={workspaceId()!}
													bindings={access.draft()}
													onChange={access.setDraft}
													subjectOptions={rolePicker.options()}
													subjectLabel={rolePicker.label}
													onLoadMoreSubjects={rolePicker.onLoadMore}
													isLoadingMoreSubjects={rolePicker.isLoadingMore}
													onSearchSubjects={rolePicker.onSearch}
													subjectPlaceholder="Select a role"
													scopeRoleId={(roleId) => roleId}
													addLabel="Add role"
													emptyText="No roles assigned."
													readOnly={!canEditAccess()}
													footer={() => (
														<a
															href="/workspace/roles/new"
															target="_blank"
															rel="noopener noreferrer"
															class="text-primary text-xs hover:underline self-start"
														>
															or create a new role &rarr;
														</a>
													)}
												/>
												<Show when={canEditAccess()}>
													<div class="flex items-center justify-end gap-3">
														<Show when={access.isDirty()}>
															<span class="text-grey text-sm">Unsaved changes</span>
															<Button
																variant={ButtonVariant.Outlined}
																onClick={access.discard}
															>
																Discard
															</Button>
														</Show>
														<Button
															variant={ButtonVariant.Contained}
															onClick={() => saveBindings().catch(() => {})}
															loading={isSaving()}
															disabled={!access.isDirty()}
														>
															Save Changes
														</Button>
													</div>
												</Show>
											</div>

											<Show when={pendingDelete()}>
												<div class="flex flex-col gap-3 p-3 border border-error/40 rounded-xs">
													<p class="text-white text-sm">
														{row().kind === "invite"
															? `Revoke the invite for ${row().title}?`
															: `Remove ${row().title} from this workspace?`}
													</p>
													<div class="flex gap-2 justify-end">
														<Button
															variant={ButtonVariant.Outlined}
															onClick={() => setPendingDelete(false)}
														>
															Cancel
														</Button>
														<Button
															variant={ButtonVariant.Contained}
															loading={isRemoving()}
															onClick={() => removePerson().catch(() => {})}
														>
															{row().kind === "invite" ? "Revoke" : "Remove"}
														</Button>
													</div>
												</div>
											</Show>
										</div>
									)}
								</Show>
							</div>
						</div>
					</div>
					<Pagination
						state={pagination}
						loading={membersQuery.isFetching}
						pinnedCount={pinnedCount()}
						showPageSizeSelector={false}
						showGoToPage={false}
					/>
				</PageContainerBody>
			</PageContainer>
		</>
	);
};

export const Route = createFileRoute("/_logged-in/_workspaced/workspace/members")({
	validateSearch: paginationSearch,
	component: ManageWorkspace,
});
