import { createFileRoute } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import { createEffect, createMemo, createSignal, Show, Suspense } from "solid-js";
import {
	Button,
	ButtonVariant,
	InputDropdown,
	PageContainer,
	PageContainerBody,
	Pagination,
	Table,
	useToast,
	UserSearchInput,
	Initials,
} from "~/components";
import { FiEdit2, FiPlus, FiTrash } from "solid-icons/fi";
import { useNavigate } from "@tanstack/solid-router";
import { createAuthenticatedAction, createFormAction, useAuthState, createPaginationState } from "~/hooks";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { UpdateUserRolesInWorkspaceRequest } from "~/bindings/UpdateUserRolesInWorkspaceRequest";
import { RemoveUserFromWorkspaceResponse } from "~/bindings/RemoveUserFromWorkspaceResponse";
import { WithId } from "~/bindings/WithId";
import { BasicUserInfo } from "~/bindings/BasicUserInfo";
import { httpRequest } from "~/utils/http-request";
import WorkspaceHeader from "./-components/workspace-header";
import { EditUserRoles } from "./-components/edit-user-roles";
import { useWorkspaceInfoQuery, useAllRolesQuery, useMembersQuery } from "~/hooks/fetch";
import { useQueryClient } from "@tanstack/solid-query";
import { memberKeys } from "~/hooks/query-keys";

const ManageWorkspace = () => {
	const [authState] = useAuthState();
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

	const { execute: deleteUser } = createAuthenticatedAction(async ({ workspaceId }) => {
		const userId = userToDelete();

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
		setShouldDelete(false);
		setUserToDelete(null);
		refetchMembers();
	});

	const roleNameMap = createMemo(() => {
		return new Map((rolesQuery.data?.roles || []).map((r) => [r.id, r.name]));
	});

	// Separate state for input fields and added members
	const [selectedUser, setSelectedUser] = createSignal<WithId<BasicUserInfo> | null>(null);
	const [currentRoleId, setCurrentRoleId] = createSignal("");
	const [shouldDelete, setShouldDelete] = createSignal(false);
	const [userToDelete, setUserToDelete] = createSignal<string | null>(null);
	const [editingMember, setEditingMember] = createSignal<{
		userId: string;
		userName: string;
		roleIds: string[];
	} | null>(null);

	const handleUserSelect = (user: WithId<BasicUserInfo>) => {
		setSelectedUser(user);
	};

	const { onSubmit: handleAddMember, isLoading: isSubmitting } = createFormAction(
		async ({ workspaceId }) => {
			const user = selectedUser();
			const roleId = currentRoleId().trim();

			const requestBody: UpdateUserRolesInWorkspaceRequest = {
				roles: [roleId],
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
			setCurrentRoleId("");
			refetchMembers();
		},
		() => {
			const user = selectedUser();
			const roleId = currentRoleId().trim();
			if (!user || !roleId) {
				toast("Please select a user and role", "error");
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
				<PageContainerBody class="flex flex-col justify-between gap-8">
					<div class="flex flex-col gap-6">
						<div class="flex flex-col gap-4">
							<form class="p-lg bg-secondary-light rounded-xs" onSubmit={handleAddMember}>
								<h1 class="text-lg mb-3">Add Someone to {workspaceInfoQuery.data?.name}</h1>

								<div class="flex flex-col items-start justify-center gap-2 w-full">
									<div class="flex items-center justify-center gap-3 w-full">
										<UserSearchInput
											placeholder="Search for user by name or username..."
											class="flex-2"
											onUserSelect={handleUserSelect}
										/>
										<InputDropdown
											placeholder="Add Roles"
											styleVariant="medium"
											class="flex-1"
											options={
												rolesQuery.data?.roles.map((role) => ({
													label: role.name,
													value: role.id,
												})) || []
											}
											value={currentRoleId()}
											onSelect={(value) => setCurrentRoleId(value)}
										/>
									</div>
								</div>

								<div class="w-full flex justify-end mt-4">
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
								<Table
									column_grids={["flex-6", "flex-3", "flex-3"]}
									headings={["User", "Roles", "Actions"]}
									rows={membersQuery.data?.members || []}
									renderRow={(member) => {
										const memberRoleIds = member.roleIds;
										const memberRoleNames = memberRoleIds
											.map((roleId) => roleNameMap().get(roleId))
											.filter(Boolean)
											.join(", ");

										if (membersQuery.isLoading) {
											return (
												<tr class="border border-border-color min-h-10 flex items-center justify-center w-full px-xl bg-secondary-light last-of-type:rounded-b-xs">
													<td colspan="3">Loading...</td>
												</tr>
											);
										}

										if (!membersQuery.data?.members || membersQuery.data.members.length <= 0) {
											return (
												<tr class="border border-border-color min-h-10 flex items-center justify-center w-full px-xl bg-secondary-light last-of-type:rounded-b-xs">
													<td colspan="3">No members found.</td>
												</tr>
											);
										}

										const isEditing = editingMember()?.userId === member.userId;

										return (
											<>
												{isEditing ? (
													<tr class="table-row">
														<td class="w-full" colspan={3}>
															<EditUserRoles
																userName={editingMember()!.userName}
																userId={editingMember()!.userId}
																workspaceId={workspaceId() || ""}
																currentRoles={
																	editingMember()!.roleIds.map((roleId) => {
																		const role = rolesQuery.data?.roles.find(
																			(r) => r.id === roleId
																		);
																		return {
																			id: roleId,
																			name: role?.name || roleId,
																		};
																	}) || []
																}
																availableRoles={
																	rolesQuery.data?.roles.map((role) => ({
																		id: role.id,
																		name: role.name,
																	})) || []
																}
																onSave={(_roleIds: string[]) => {
																	setEditingMember(null);
																	refetchMembers();
																}}
																onClose={() => {
																	setEditingMember(null);
																}}
															/>
														</td>
													</tr>
												) : (
													<tr role="row" class="table-row">
														<td
															role="cell"
															class="flex items-center justify-start flex-6 gap-2 min-w-0"
														>
															<Initials
																size="xs"
																firstName={member.firstName}
																lastName={member.lastName}
															/>
															<div class="flex flex-col min-w-0">
																<span class="text-white font-medium truncate">
																	{member.fullName}
																</span>
																<span class="text-grey text-xs truncate">
																	@{member.username}
																</span>
															</div>
														</td>
														<td
															role="cell"
															class="flex items-center justify-start flex-3 min-w-0"
														>
															<span class={memberRoleNames ? "" : "text-grey italic"}>
																{memberRoleNames || "No roles"}
															</span>
														</td>
														<td class="flex items-center justify-center flex-3">
															{shouldDelete() && userToDelete() === member.userId ? (
																<>
																	<div class="flex gap-2">
																		<button
																			class="text-red-500"
																			onClick={async (e: MouseEvent) => {
																				e.stopPropagation();
																				await deleteUser().catch(() => {});
																			}}
																		>
																			Delete
																		</button>
																		<button
																			onClick={() => {
																				setShouldDelete(false);
																				setUserToDelete(null);
																			}}
																		>
																			Cancel
																		</button>
																	</div>
																</>
															) : (
																<>
																	<button
																		aria-label="Edit member roles"
																		onClick={() => {
																			setEditingMember({
																				userId: member.userId,
																				userName: member.fullName,
																				roleIds: member.roleIds,
																			});
																		}}
																		class="text-grey hover:bg-white/10 p-1 rounded transition-colors cursor-pointer"
																	>
																		<FiEdit2 size={18} />
																	</button>
																	<button
																		aria-label="Remove member"
																		onClick={(e) => {
																			e.stopPropagation();
																			setUserToDelete(member.userId);
																			setShouldDelete(true);
																		}}
																		class="text-error hover:bg-white/10 p-1 rounded transition-colors cursor-pointer"
																	>
																		<FiTrash size={18} />
																	</button>
																</>
															)}
														</td>
													</tr>
												)}
											</>
										);
									}}
								/>
							</Suspense>
						</div>
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
