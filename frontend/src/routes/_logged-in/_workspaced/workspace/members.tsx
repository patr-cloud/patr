import { createFileRoute } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import { createEffect, createMemo, createSignal, For, Show, Suspense } from "solid-js";
import {
	Button,
	ButtonVariant,
	InputDropdownCheckBox,
	PageContainer,
	PageContainerBody,
	Pagination,
	useToast,
	UserSearchInput,
	Initials,
} from "~/components";
import { FiCheck, FiChevronRight, FiEdit2, FiPlus, FiTrash, FiX } from "solid-icons/fi";
import { useNavigate } from "@tanstack/solid-router";
import { createAuthenticatedAction, createFormAction, createPaginationState } from "~/hooks";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { UpdateUserRolesInWorkspaceRequest } from "~/bindings/UpdateUserRolesInWorkspaceRequest";
import { RemoveUserFromWorkspaceResponse } from "~/bindings/RemoveUserFromWorkspaceResponse";
import { WithId } from "~/bindings/WithId";
import { BasicUserInfo } from "~/bindings/BasicUserInfo";
import { httpRequest } from "~/utils/http-request";
import WorkspaceHeader from "./-components/workspace-header";
import { useWorkspaceInfoQuery, useAllRolesQuery, useMembersQuery } from "~/hooks/fetch";
import { useQueryClient } from "@tanstack/solid-query";
import { memberKeys } from "~/hooks/query-keys";

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
	const rolesQuery = useAllRolesQuery();
	const membersQuery = useMembersQuery(
		() => search().page,
		() => search().count
	);

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

	const [selectedUser, setSelectedUser] = createSignal<WithId<BasicUserInfo> | null>(null);
	const [currentRoleIds, setCurrentRoleIds] = createSignal<string[]>([]);
	const [selectedMemberId, setSelectedMemberId] = createSignal<string | null>(null);
	const [isEditingRoles, setIsEditingRoles] = createSignal(false);
	const [editingRoleIds, setEditingRoleIds] = createSignal<string[]>([]);
	const [pendingDeleteUserId, setPendingDeleteUserId] = createSignal<string | null>(null);

	createEffect(() => {
		const members = membersQuery.data?.members;
		if (!members || members.length === 0) return;
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

		const requestBody: UpdateUserRolesInWorkspaceRequest = {
			roles: editingRoleIds(),
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
		return membersQuery.data?.members.find((m) => m.userId === id) ?? null;
	});

	const handleUserSelect = (user: WithId<BasicUserInfo>) => {
		setSelectedUser(user);
	};

	const { onSubmit: handleAddMember, isLoading: isSubmitting } = createFormAction(
		async ({ workspaceId }) => {
			const user = selectedUser();
			const roleIds = currentRoleIds();

			const requestBody: UpdateUserRolesInWorkspaceRequest = {
				roles: roleIds,
			};

			const response = await httpRequest(
				`${import.meta.env.VITE_BASE_URL}/api/workspace/${workspaceId}/rbac/user/${user!.id}`,
				{
					method: "POST",
					body: JSON.stringify(requestBody),
				}
			);

			if (!response.ok) {
				console.error("Failed to add user:", response.data.error);
				toast(response.data.error || "Failed to add user to workspace", "error");
				return;
			}

			toast("User added successfully", "success");
			setCurrentRoleIds([]);
			refetchMembers();
		},
		() => {
			const user = selectedUser();
			const roleIds = currentRoleIds();
			if (!user || roleIds.length === 0) {
				toast("Please select a user and at least one role", "error");
				return false;
			}
			return true;
		}
	);

	return (
		<>
			<Title>Workspace Members | Patr</Title>
			<PageContainer>
				<WorkspaceHeader workspaceName={workspaceInfoQuery.data?.name} activeTab="members" />
				<PageContainerBody class="flex flex-col justify-between gap-4">
					<div class="flex flex-col gap-6 flex-1">
						<form class="p-lg bg-secondary-light rounded-xs" onSubmit={handleAddMember}>
							<h1 class="text-lg mb-3">Add Someone to {workspaceInfoQuery.data?.name}</h1>

							<div class="flex items-center justify-center gap-3 w-full">
								<UserSearchInput
									placeholder="Paste user ID"
									class="flex-2"
									onUserSelect={handleUserSelect}
								/>
								<InputDropdownCheckBox
									placeholder={
										currentRoleIds().length > 0
											? `${currentRoleIds().length} role${currentRoleIds().length === 1 ? "" : "s"} selected`
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
									checked={currentRoleIds()}
									onToggle={(value) =>
										setCurrentRoleIds((prev) =>
											prev.includes(value) ? prev.filter((id) => id !== value) : [...prev, value]
										)
									}
								/>
								<Button
									type="submit"
									variant={ButtonVariant.Contained}
									class="h-full flex items-center gap-2"
									disabled={isSubmitting()}
									loading={isSubmitting()}
									loadingContent={() => <span>Adding...</span>}
								>
									<FiPlus size={16} />
									Add Member
								</Button>
							</div>
						</form>

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
										when={(membersQuery.data?.members?.length ?? 0) > 0}
										fallback={
											<div class="flex items-center justify-center py-16 text-grey">
												<span class="text-sm">No members found.</span>
											</div>
										}
									>
										<ul class="flex flex-col gap-2 p-2">
											<For each={membersQuery.data?.members || []}>
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
																	{member.userId}
																</span>
															</div>
															<div class="px-3 py-1 border border-border-color rounded-xs text-xs text-grey">
																{member.roleIds.length}&nbsp;
																{member.roleIds.length === 1 ? "role" : "roles"}
															</div>
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
														<Show when={!isEditingRoles() && !isPendingDelete()}>
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
														<span class="text-grey text-sm">{member().userId}</span>
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
