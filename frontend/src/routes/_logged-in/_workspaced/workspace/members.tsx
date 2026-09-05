import { createFileRoute } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import { createEffect, createMemo, createSignal, For, Show, Suspense } from "solid-js";
import {
	Button,
	ButtonVariant,
	Input,
	InputType,
	InputDropdownCheckBox,
	PageContainer,
	PageContainerBody,
	Pagination,
	useToast,
	Initials,
} from "~/components";
import { FiCheck, FiChevronRight, FiCopy, FiEdit2, FiPlus, FiTrash, FiX } from "solid-icons/fi";
import { useNavigate } from "@tanstack/solid-router";
import { createAuthenticatedAction, createFormAction, createPaginationState, useIsAllowed } from "~/hooks";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { UpdateUserRolesInWorkspaceRequest } from "~/bindings/UpdateUserRolesInWorkspaceRequest";
import { RemoveUserFromWorkspaceResponse } from "~/bindings/RemoveUserFromWorkspaceResponse";
import { InviteUserToWorkspaceRequest } from "~/bindings/InviteUserToWorkspaceRequest";
import { InviteUserToWorkspaceResponse } from "~/bindings/InviteUserToWorkspaceResponse";
import { ResendWorkspaceInviteResponse } from "~/bindings/ResendWorkspaceInviteResponse";
import { UpdateWorkspaceInviteRolesRequest } from "~/bindings/UpdateWorkspaceInviteRolesRequest";
import { RoleBindingGrant } from "~/bindings/RoleBindingGrant";
import { httpRequest } from "~/utils/http-request";
import WorkspaceHeader from "./-components/workspace-header";
import { useWorkspaceInfoQuery, useAllRolesQuery, useMembersQuery, useInvitesQuery } from "~/hooks/fetch";
import { useQueryClient } from "@tanstack/solid-query";
import { memberKeys, inviteKeys } from "~/hooks/query-keys";

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
	// The Add-Member role picker is a single-page checkbox list, so it needs
	// every role in one shot — passing the largest allowed page size avoids a
	// second round trip for workspaces with the typical 30-50 roles. If your
	// workspace exceeds 100 roles, swap this for a paginated dropdown.
	const rolesQuery = useAllRolesQuery(
		() => undefined,
		() => "100"
	);
	const membersQuery = useMembersQuery(
		() => search().page,
		() => search().count
	);
	const invitesQuery = useInvitesQuery();
	const canModifyMembers = useIsAllowed("modifyRoles", "edit");

	// The list endpoint includes the owner, flagged `isOwner`. Pin them first
	// so the row order doesn't shuffle as other members come and go.
	const displayedMembers = createMemo(() => {
		const raw = membersQuery.data?.members ?? [];
		return [...raw].sort((a, b) => Number(b.isOwner) - Number(a.isOwner));
	});

	createEffect(() => {
		const totalCount = membersQuery.data?.totalCount;
		if (totalCount !== undefined) {
			pagination.setTotalCount(totalCount);
		}
	});

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

	const [inviteEmail, setInviteEmail] = createSignal("");
	const [inviteRoleIds, setInviteRoleIds] = createSignal<string[]>([]);
	const [selectedMemberId, setSelectedMemberId] = createSignal<string | null>(null);
	const [isEditingRoles, setIsEditingRoles] = createSignal(false);
	const [editingRoleIds, setEditingRoleIds] = createSignal<string[]>([]);
	const [pendingDeleteUserId, setPendingDeleteUserId] = createSignal<string | null>(null);

	createEffect(() => {
		const members = displayedMembers();
		if (members.length === 0) return;
		if (selectedMemberId() === null || !members.some((m) => m.userId === selectedMemberId())) {
			setSelectedMemberId(members[0].userId);
		}
	});

	const { execute: deleteUser } = createAuthenticatedAction(async ({ workspaceId }) => {
		const userId = pendingDeleteUserId();

		if (!userId) {
			toast("No user selected for deletion", "error");
			return;
		}

		const response = await httpRequest<RemoveUserFromWorkspaceResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${workspaceId}/rbac/user/${userId}`,
			{
				method: "DELETE",
			}
		);

		if (!response.ok) {
			console.error("Failed to delete user:", response.data.error);
			toast("Failed to delete user", "error");
			return;
		}

		toast("User removed successfully", "success");
		setPendingDeleteUserId(null);
		if (selectedMemberId() === userId) {
			setSelectedMemberId(null);
		}
		refetchMembers();
	});

	const { execute: saveRoles, isLoading: isSavingRoles } = createAuthenticatedAction(async ({ workspaceId }) => {
		const userId = selectedMemberId();
		if (!userId) return;

		// Scope selection lands with the members-page rework; until then every
		// grant is workspace-wide, matching the pre-migration behaviour.
		const requestBody: UpdateUserRolesInWorkspaceRequest = {
			// Grants sit at the workspace root until the scope picker lands.
			roles: editingRoleIds().map((roleId) => ({ roleId, resourceId: workspaceId })),
		};

		const response = await httpRequest(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${workspaceId}/rbac/user/${userId}`,
			{
				method: "POST",
				body: JSON.stringify(requestBody),
			}
		);

		if (!response.ok) {
			console.error("Failed to update roles:", response.data.error);
			toast("Failed to update roles", "error");
			return;
		}

		toast("Roles updated successfully", "success");
		setIsEditingRoles(false);
		refetchMembers();
	});

	const roleNameMap = createMemo(() => {
		return new Map((rolesQuery.data?.roles || []).map((r) => [r.id, r.name]));
	});

	const selectedMember = createMemo(() => {
		const id = selectedMemberId();
		if (!id) return null;
		return displayedMembers().find((m) => m.userId === id) ?? null;
	});

	const { onSubmit: handleInvite, isLoading: isSubmitting } = createFormAction(
		async ({ workspaceId }) => {
			const requestBody: InviteUserToWorkspaceRequest = {
				email: inviteEmail().trim(),
				roles: inviteRoleIds().map((roleId) => ({ roleId, resourceId: workspaceId })),
			};

			const response = await httpRequest<InviteUserToWorkspaceResponse>(
				`${import.meta.env.VITE_BASE_URL}/api/workspace/${workspaceId}/rbac/user/invite`,
				{
					method: "POST",
					body: JSON.stringify(requestBody),
				}
			);

			if (!response.ok) {
				console.error("Failed to invite user:", response.data.error);
				const err = response.data.error;
				toast(
					err === "userAlreadyMember"
						? "That email already belongs to a member of this workspace"
						: err === "inviteAlreadyExists"
							? "That email has already been invited — edit or revoke it below"
							: "Failed to send invite",
					"error"
				);
				return;
			}

			// Stash the returned link so a "copy link" button can appear on the
			// new invite. The token is only returned here (it's stored hashed).
			setInviteLinks((prev) => ({ ...prev, [response.data.id]: response.data.acceptUrl }));
			toast("Invite sent", "success");
			setInviteEmail("");
			setInviteRoleIds([]);
			refetchInvites();
		},
		() => {
			if (!inviteEmail().trim() || inviteRoleIds().length === 0) {
				toast("Please enter an email and select at least one role", "error");
				return false;
			}
			return true;
		}
	);

	const [pendingRevokeId, setPendingRevokeId] = createSignal<string | null>(null);

	const { execute: revokeInvite } = createAuthenticatedAction(async ({ workspaceId }) => {
		const inviteId = pendingRevokeId();
		if (!inviteId) return;

		const response = await httpRequest(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${workspaceId}/rbac/user/invite/${inviteId}`,
			{ method: "DELETE" }
		);

		if (!response.ok) {
			console.error("Failed to revoke invite:", response.data.error);
			toast("Failed to revoke invite", "error");
			return;
		}

		toast("Invite revoked", "success");
		setPendingRevokeId(null);
		refetchInvites();
	});

	const [editingInviteId, setEditingInviteId] = createSignal<string | null>(null);
	const [editingInviteRoleIds, setEditingInviteRoleIds] = createSignal<string[]>([]);

	const beginEditInvite = (inviteId: string, grants: RoleBindingGrant[]) => {
		setPendingRevokeId(null);
		setEditingInviteId(inviteId);
		setEditingInviteRoleIds(grants.map((grant) => grant.roleId));
	};

	const cancelEditInvite = () => {
		setEditingInviteId(null);
		setEditingInviteRoleIds([]);
	};

	const { execute: saveInviteRoles, isLoading: isSavingInvite } = createAuthenticatedAction(
		async ({ workspaceId }) => {
			const inviteId = editingInviteId();
			if (!inviteId) return;

			const body: UpdateWorkspaceInviteRolesRequest = {
				roles: editingInviteRoleIds().map((roleId) => ({ roleId, resourceId: workspaceId })),
			};
			const response = await httpRequest(
				`${import.meta.env.VITE_BASE_URL}/api/workspace/${workspaceId}/rbac/user/invite/${inviteId}`,
				{ method: "PATCH", body: JSON.stringify(body) }
			);

			if (!response.ok) {
				console.error("Failed to update invite roles:", response.data.error);
				toast("Failed to update invite", "error");
				return;
			}

			toast("Invite updated", "success");
			cancelEditInvite();
			refetchInvites();
		}
	);

	// Accept links keyed by invite id, populated when an invite is created or
	// resent (the only times the plaintext token is returned). Lets us show a
	// "copy link" button for those invites; it's not available after a reload.
	const [inviteLinks, setInviteLinks] = createSignal<Record<string, string>>({});

	const resendInvite = async (inviteId: string) => {
		const wsId = workspaceId();
		if (!wsId) return;

		const response = await httpRequest<ResendWorkspaceInviteResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/rbac/user/invite/${inviteId}/resend`,
			{ method: "POST" }
		);

		if (!response.ok) {
			console.error("Failed to resend invite:", response.data.error);
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

	const inviteRoleNames = (grants: RoleBindingGrant[]) =>
		grants.map((grant) => roleNameMap().get(grant.roleId) || grant.roleId);

	// An expired invite is still listed for a while so it can be resent, but its
	// link no longer works — say so rather than showing it as merely pending.
	const isExpired = (expiry: Date) => new Date(expiry).getTime() <= Date.now();

	return (
		<>
			<Title>Workspace Members | Patr</Title>
			<PageContainer>
				<WorkspaceHeader workspaceName={workspaceInfoQuery.data?.name} activeTab="members" />
				<PageContainerBody class="flex flex-col justify-between gap-4">
					<div class="flex flex-col gap-6 flex-1">
						<Show when={canModifyMembers()}>
							<form class="p-lg bg-secondary-light rounded-xs" onSubmit={handleInvite}>
								<h1 class="text-lg mb-3">Invite Someone to {workspaceInfoQuery.data?.name}</h1>

								<div class="flex items-center justify-center gap-3 w-full">
									<Input
										type={InputType.Email}
										placeholder="Email address to invite..."

										class="flex-2"
										value={inviteEmail()}
										onInput={(e) => setInviteEmail(e.currentTarget.value)}
									/>
									<InputDropdownCheckBox
										placeholder={
											inviteRoleIds().length > 0
												? `${inviteRoleIds().length} role${inviteRoleIds().length === 1 ? "" : "s"} selected`
												: "Add roles..."
										}
										styleVariant="medium"
										class="flex-1"
										options={
											rolesQuery.data?.roles.map((role) => ({
												label: role.name,
												value: role.id,
											})) || []
										}
										checked={inviteRoleIds()}
										onToggle={(value) =>
											setInviteRoleIds((prev) =>
												prev.includes(value)
													? prev.filter((id) => id !== value)
													: [...prev, value]
											)
										}
									/>
									<Button
										type="submit"
										variant={ButtonVariant.Contained}
										class="h-full flex items-center gap-2"
										disabled={isSubmitting()}
										loading={isSubmitting()}
										loadingContent={() => <span>Sending...</span>}
									>
										<FiPlus size={16} />
										Send Invite
									</Button>
								</div>
							</form>
						</Show>

						<Show when={(invitesQuery.data?.length ?? 0) > 0}>
							<div class="flex flex-col gap-3 p-lg bg-secondary-light rounded-xs">
								<h2 class="text-lg">Pending invitations</h2>
								<ul class="flex flex-col gap-2">
									<For each={invitesQuery.data ?? []}>
										{(invite) => {
											const isEditing = () => editingInviteId() === invite.id;
											const isPendingRevoke = () => pendingRevokeId() === invite.id;
											return (
												<li class="flex flex-col gap-3 px-lg py-3 rounded-xs border border-border-color">
													<div class="flex items-center gap-4">
														<div class="flex flex-col min-w-0 flex-1">
															<span class="flex items-center gap-2 min-w-0">
																<span class="text-white font-medium truncate">
																	{invite.email}
																</span>
																<Show when={isExpired(invite.expiry)}>
																	<span class="shrink-0 text-warning text-xs border border-warning-light rounded-xs px-2 py-px">
																		Expired
																	</span>
																</Show>
															</span>
															<span class="text-grey text-xs truncate">
																{inviteRoleNames(invite.roles).join(", ") || "No roles"}
															</span>
														</div>
														<Show when={canModifyMembers() && !isEditing()}>
															<Show
																when={isPendingRevoke()}
																fallback={
																	<div class="flex items-center gap-2">
																		<Show when={inviteLinks()[invite.id]}>
																			<Button
																				variant={ButtonVariant.Outlined}
																				class="flex items-center gap-2"
																				onClick={() =>
																					copyInviteLink(invite.id)
																				}
																			>
																				<FiCopy size={14} />
																				Copy link
																			</Button>
																		</Show>
																		<Button
																			variant={ButtonVariant.Outlined}
																			class="flex items-center gap-2"
																			onClick={() =>
																				beginEditInvite(invite.id, invite.roles)
																			}
																		>
																			<FiEdit2 size={14} />
																			Edit roles
																		</Button>
																		<Button
																			variant={ButtonVariant.Outlined}
																			onClick={() => resendInvite(invite.id)}
																		>
																			Resend
																		</Button>
																		<button
																			aria-label="Revoke invite"
																			onClick={() =>
																				setPendingRevokeId(invite.id)
																			}
																			class="text-error border border-border-color hover:bg-white/10 p-2 rounded-xs transition-colors cursor-pointer"
																		>
																			<FiTrash size={16} />
																		</button>
																	</div>
																}
															>
																<div class="flex items-center gap-2">
																	<Button
																		variant={ButtonVariant.Contained}
																		onClick={() => revokeInvite().catch(() => {})}
																	>
																		Revoke
																	</Button>
																	<Button
																		variant={ButtonVariant.Outlined}
																		onClick={() => setPendingRevokeId(null)}
																	>
																		Cancel
																	</Button>
																</div>
															</Show>
														</Show>
													</div>
													<Show when={isEditing()}>
														<div class="flex items-center gap-2">
															<InputDropdownCheckBox
																placeholder={
																	editingInviteRoleIds().length > 0
																		? `${editingInviteRoleIds().length} role${editingInviteRoleIds().length === 1 ? "" : "s"} selected`
																		: "Select roles..."
																}
																styleVariant="medium"
																class="flex-1"
																options={
																	rolesQuery.data?.roles.map((role) => ({
																		label: role.name,
																		value: role.id,
																	})) || []
																}
																checked={editingInviteRoleIds()}
																onToggle={(value) =>
																	setEditingInviteRoleIds((prev) =>
																		prev.includes(value)
																			? prev.filter((id) => id !== value)
																			: [...prev, value]
																	)
																}
															/>
															<Button
																variant={ButtonVariant.Contained}
																loading={isSavingInvite()}
																onClick={() => saveInviteRoles().catch(() => {})}
															>
																Save
															</Button>
															<Button
																variant={ButtonVariant.Outlined}
																onClick={cancelEditInvite}
															>
																Cancel
															</Button>
														</div>
													</Show>
												</li>
											);
										}}
									</For>
								</ul>
							</div>
						</Show>

						<Suspense
							fallback={
								<div class="flex items-center justify-center gap-2 py-16 text-grey">
									<span class="text-sm">Loading members...</span>
								</div>
							}
						>
							<div class="flex flex-1 flex-col lg:flex-row gap-6 items-start">
								<div
									class={`flex-2 w-full bg-secondary-light rounded-xs overflow-hidden transition-opacity ${
										isEditingRoles() ? "opacity-50 pointer-events-none" : ""
									}`}
								>
									<Show
										when={displayedMembers().length > 0}
										fallback={
											<div class="flex items-center justify-center py-16 text-grey">
												<span class="text-sm">No members found.</span>
											</div>
										}
									>
										<ul class="flex flex-col gap-2 p-2">
											<For each={displayedMembers()}>
												{(member) => {
													const isSelected = () => selectedMemberId() === member.userId;
													return (
														<li
															role="button"
															tabIndex={0}
															onClick={() => {
																setSelectedMemberId(member.userId);
																setIsEditingRoles(false);
																setPendingDeleteUserId(null);
															}}
															onKeyDown={(e) => {
																if (e.key === "Enter" || e.key === " ") {
																	e.preventDefault();
																	setSelectedMemberId(member.userId);
																	setIsEditingRoles(false);
																	setPendingDeleteUserId(null);
																}
															}}
															class={`relative flex items-center gap-4 px-lg py-4 cursor-pointer rounded-xs border border-border-color border-l-2 transition-colors hover:bg-secondary-medium ${
																isSelected()
																	? "border-l-primary bg-secondary-medium"
																	: "border-l-border-color"
															}`}
														>
															<Initials
																size="sm"
																firstName={member.firstName}
																lastName={member.lastName}
															/>
															<div class="flex flex-col min-w-0 flex-1">
																<span class="text-white font-medium truncate">
																	{member.fullName}
																</span>
																<span class="text-grey text-xs truncate">
																	{member.email}
																</span>
															</div>
															<Show
																when={!member.isOwner}
																fallback={
																	<div class="px-3 py-1 border border-primary rounded-xs text-xs text-primary">
																		Owner
																	</div>
																}
															>
																<div class="px-3 py-1 border border-border-color rounded-xs text-xs text-grey">
																	{member.roleIds.length}&nbsp;
																	{member.roleIds.length === 1 ? "role" : "roles"}
																</div>
															</Show>
															<FiChevronRight size={18} class="text-grey shrink-0" />
														</li>
													);
												}}
											</For>
										</ul>
									</Show>
								</div>

								<div class="flex-1 w-full lg:sticky lg:top-4">
									<Show
										when={selectedMember()}
										fallback={
											<div class="bg-secondary-light rounded-xs p-lg text-grey text-sm flex items-center justify-center min-h-50">
												Select a member to see details.
											</div>
										}
									>
										{(member) => {
											const displayedRoleIds = createMemo(() =>
												isEditingRoles() ? editingRoleIds() : member().roleIds
											);
											const displayedRoles = createMemo(() =>
												displayedRoleIds().map((roleId) => ({
													id: roleId,
													name: roleNameMap().get(roleId) || roleId,
												}))
											);

											const isPendingDelete = () => pendingDeleteUserId() === member().userId;

											const beginEditing = () => {
												setEditingRoleIds([...member().roleIds]);
												setIsEditingRoles(true);
											};

											const cancelEditing = () => {
												setIsEditingRoles(false);
												setEditingRoleIds([]);
											};

											const removeEditingRole = (roleId: string) => {
												setEditingRoleIds((prev) => prev.filter((id) => id !== roleId));
											};

											const toggleEditingRole = (roleId: string) => {
												setEditingRoleIds((prev) =>
													prev.includes(roleId)
														? prev.filter((id) => id !== roleId)
														: [...prev, roleId]
												);
											};

											return (
												<div class="bg-secondary-light rounded-xs p-lg flex flex-col gap-5">
													<div class="flex items-start justify-between gap-3">
														<Initials
															size="lg"
															firstName={member().firstName}
															lastName={member().lastName}
														/>
														<Show
															when={
																!isEditingRoles() &&
																!isPendingDelete() &&
																!member().isOwner &&
																canModifyMembers()
															}
														>
															<div class="flex items-center gap-2">
																<Button
																	variant={ButtonVariant.Outlined}
																	onClick={beginEditing}
																	class="flex items-center gap-2"
																>
																	<FiEdit2 size={14} />
																	Edit roles
																</Button>
																<button
																	aria-label="Remove member"
																	onClick={() =>
																		setPendingDeleteUserId(member().userId)
																	}
																	class="text-error border border-border-color hover:bg-white/10 p-2 rounded-xs transition-colors cursor-pointer"
																>
																	<FiTrash size={16} />
																</button>
															</div>
														</Show>
														<Show when={isEditingRoles()}>
															<div class="flex items-center gap-2">
																<Button
																	variant={ButtonVariant.Contained}
																	onClick={() => saveRoles().catch(() => {})}
																	loading={isSavingRoles()}
																	class="flex items-center gap-2"
																>
																	<FiCheck size={14} />
																	Save
																</Button>
																<Button
																	variant={ButtonVariant.Outlined}
																	onClick={cancelEditing}
																	class="flex items-center gap-2"
																>
																	<FiX size={14} />
																	Cancel
																</Button>
															</div>
														</Show>
													</div>

													<div class="flex flex-col gap-1">
														<span class="text-white text-xl font-medium">
															{member().fullName}
														</span>
														<span class="text-grey text-sm">{member().email}</span>
													</div>

													<div class="flex flex-col gap-3">
														<div class="flex items-center justify-between">
															<h3 class="text-white text-sm font-medium">
																Assigned roles
															</h3>
															<span class="text-grey text-xs">
																{displayedRoles().length}
															</span>
														</div>
														<Show
															when={displayedRoles().length > 0}
															fallback={
																<p class="text-grey text-sm italic">
																	No roles assigned.
																</p>
															}
														>
															<div class="flex flex-wrap gap-2 max-h-52 overflow-y-auto pr-1">
																<For each={displayedRoles()}>
																	{(role) => (
																		<span class="inline-flex items-center gap-2 px-3 py-1 bg-secondary-light border border-border-color rounded-xs text-white text-xs">
																			{role.name}
																			<Show when={isEditingRoles()}>
																				<button
																					type="button"
																					aria-label={`Remove ${role.name}`}
																					onClick={() =>
																						removeEditingRole(role.id)
																					}
																					class="text-grey hover:text-error transition-colors cursor-pointer"
																				>
																					<FiX size={12} />
																				</button>
																			</Show>
																		</span>
																	)}
																</For>
															</div>
														</Show>

														<Show when={isEditingRoles()}>
															<div class="flex flex-col gap-2 p-3 border border-dashed border-border-color rounded-xs">
																<InputDropdownCheckBox
																	placeholder="+ Add role..."
																	styleVariant="medium"
																	options={
																		rolesQuery.data?.roles.map((role) => ({
																			label: role.name,
																			value: role.id,
																		})) || []
																	}
																	checked={editingRoleIds()}
																	onToggle={toggleEditingRole}
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
														</Show>
													</div>

													<Show when={isPendingDelete()}>
														<div class="flex flex-col gap-3 p-3 border border-error/40 rounded-xs">
															<p class="text-white text-sm">
																Remove {member().fullName} from this workspace?
															</p>
															<div class="flex gap-2 justify-end">
																<Button
																	variant={ButtonVariant.Outlined}
																	onClick={() => setPendingDeleteUserId(null)}
																>
																	Cancel
																</Button>
																<Button
																	variant={ButtonVariant.Contained}
																	onClick={() => deleteUser().catch(() => {})}
																>
																	Remove
																</Button>
															</div>
														</div>
													</Show>
												</div>
											);
										}}
									</Show>
								</div>
							</div>
						</Suspense>
					</div>
					<Pagination
						state={pagination}
						loading={membersQuery.isFetching}
						showPageSizeSelector={false}
						showGoToPage={false}
					/>
				</PageContainerBody>
			</PageContainer>
		</>
	);
};

export const Route = createFileRoute("/_logged-in/_workspaced/workspace/members")({
	validateSearch: (search: Record<string, unknown>): { page?: string; count?: string } => ({
		page: (search.page as string) || undefined,
		count: (search.count as string) || undefined,
	}),
	component: ManageWorkspace,
});
