import { createResource, createSignal, Show, Suspense } from "solid-js";
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
import { createAuthenticatedAction, createFormAction, useAuthState, createPaginationState } from "~/hooks";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { GetWorkspaceInfoResponse } from "~/bindings/GetWorkspaceInfoResponse";
import { ListAllRolesResponse } from "~/bindings/ListAllRolesResponse";
import { ListUsersInWorkspaceResponse } from "~/bindings/ListUsersInWorkspaceResponse";
import { GetUserDetailsResponse } from "~/bindings/GetUserDetailsResponse";
import { UpdateUserRolesInWorkspaceRequest } from "~/bindings/UpdateUserRolesInWorkspaceRequest";
import { RemoveUserFromWorkspaceResponse } from "~/bindings/RemoveUserFromWorkspaceResponse";
import { WithId } from "~/bindings/WithId";
import { BasicUserInfo } from "~/bindings/BasicUserInfo";
import { httpRequest } from "~/utils/http-request";
import WorkspaceHeader from "~/pages/workspace/workspace-header";
import { EditUserRoles } from "~/pages/workspace/edit-user-roles";

const ManageWorkspace = () => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();
	const toast = useToast();
	const pagination = createPaginationState();
	const resourceParamsWorkspace = () => {
		return [authState(), workspaceId()] as const;
	};
	const [workspaceInfo] = createResource(resourceParamsWorkspace, async ([auth, id]) => {
		if (!auth || auth.type !== "LoggedIn" || id === "") {
			return;
		}
		const response = await httpRequest<GetWorkspaceInfoResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${id}`,
			{
				method: "GET",
				headers: {
					"Content-Type": "application/json",
					Authorization: `Bearer ${auth.accessToken}`,
				},
			}
		);
		if (!response.ok) {
			console.error("Failed to fetch workspace info:", response.data.error);
			toast("Failed to fetch workspace info", "error");
			return undefined;
		}
		return response.data;
	});

	const [roles] = createResource(resourceParamsWorkspace, async ([auth, id]) => {
		if (!auth || auth.type !== "LoggedIn" || id === "") {
			return;
		}
		const response = await httpRequest<ListAllRolesResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${id}/rbac/role`,
			{
				method: "GET",
				headers: {
					"Content-Type": "application/json",
					Authorization: `Bearer ${auth.accessToken}`,
				},
			}
		);
		if (!response.ok) {
			console.error("Failed to fetch roles:", response.data.error);
			toast("Failed to fetch roles", "error");
			return undefined;
		}
		return response.data;
	});

	const membersFetchParams = () => {
		return [authState(), workspaceId(), pagination.page(), pagination.count()] as const;
	};

	const [workspaceMembers, { refetch: refetchMembers }] = createResource(
		membersFetchParams,
		async ([auth, id, page, count]) => {
			if (!auth || auth.type !== "LoggedIn" || id === "") {
				return;
			}
			const response = await httpRequest<ListUsersInWorkspaceResponse>(
				`${import.meta.env.VITE_BASE_URL}/api/workspace/${id}/rbac/user?page=${page}&count=${count}`,
				{
					method: "GET",
					headers: {
						"Content-Type": "application/json",
						Authorization: `Bearer ${auth.accessToken}`,
					},
				}
			);
			if (!response.ok) {
				console.error("Failed to fetch workspace members:", response.data.error);
				toast("Failed to fetch workspace members", "error");
				return undefined;
			}
			pagination.setTotalCount(Number(response.headers.get("x-total-count") ?? 0));
			// Fetch user details for each user ID
			const userDetailsPromises = Object.keys(response.data.users).map(async (userId) => {
				const userResponse = await httpRequest<GetUserDetailsResponse>(
					`${import.meta.env.VITE_BASE_URL}/api/user/${userId}`,
					{
						method: "GET",
						headers: {
							"Content-Type": "application/json",
							Authorization: `Bearer ${auth.accessToken}`,
						},
					}
				);

				console.log("User response for", userId, ":", userResponse);

				if (userResponse.ok) {
					const user = userResponse.data;
					console.log("User data:", user);
					const roleIds = response.data.users[userId] || [];

					// Handle both flattened and nested response structures
					const firstName = user.firstName || "";
					const lastName = user.lastName || "";
					const username = user.username || "";
					const id = user.id || "";

					return {
						userId: id,
						userName: `${firstName} ${lastName} (@${username})`,
						roleIds: roleIds,
					};
				}
				console.error("Failed to fetch user details for", userId, ":", userResponse.data);
				return null;
			});

			const userDetails = await Promise.all(userDetailsPromises);
			return userDetails.filter((user) => user !== null);
		}
	);

	const { execute: deleteUser, isLoading: isDeleting } = createAuthenticatedAction(
		async ({ accessToken, workspaceId }) => {
			const userId = userToDelete();

			if (!userId) {
				toast("No user selected for deletion", "error");
				return;
			}

			const response = await httpRequest<RemoveUserFromWorkspaceResponse>(
				`${import.meta.env.VITE_BASE_URL}/api/workspace/${workspaceId}/rbac/user/${userId}`,
				{
					method: "DELETE",
					headers: {
						Authorization: `Bearer ${accessToken}`,
					},
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
		}
	);

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
		async ({ accessToken, workspaceId }) => {
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
		<PageContainer>
			<WorkspaceHeader workspaceName={workspaceInfo()?.name} activeTab="members" />
			<PageContainerBody class="flex flex-col justify-between gap-8">
				<div class="flex flex-col gap-6">
					<div class="flex flex-col gap-4">
						<form class="p-lg bg-secondary-light rounded-xs" onSubmit={handleAddMember}>
							<h1 class="text-lg mb-3">Add Someone to {workspaceInfo()?.name}</h1>

							<div class="flex flex-col items-start justify-center gap-2 w-full">
								<div class="flex items-center justify-center gap-3 w-full">
									<Show when={authState() && authState()!.type === "LoggedIn"} fallback={<div class="flex-2" />}>
										<UserSearchInput
											placeholder="Search for user by name or username..."
											class="flex-2"
											accessToken={(authState()! as any).accessToken}
											onUserSelect={handleUserSelect}
										/>
									</Show>
									<InputDropdown
										placeholder="Add Roles"
										styleVariant="medium"
										class="flex-1"
										options={
											roles()?.roles.map((role) => ({
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
								>
									<FiPlus size={16} />
								</Button>
							</div>
						</form>

						<Suspense fallback={<div class="text-white">Loading members...</div>}>
							<Table
								column_grids={["flex-2", "flex-1", "flex-1"]}
								headings={["User", "Roles", "Actions"]}
								rows={workspaceMembers() || []}
								renderRow={(member) => {
									const memberRoleIds = member.roleIds;
									const memberRoleNames = memberRoleIds
										.map((roleId) => roles()?.roles.find((r) => r.id === roleId)?.name)
										.filter(Boolean)
										.join(", ");

									if (workspaceMembers.loading) {
										return (
											<tr class="border border-border-color min-h-10 flex items-center justify-center w-full px-xl bg-secondary-light last-of-type:rounded-b-xs">
												<td colspan="3">Loading...</td>
											</tr>
										);
									}

									if (!workspaceMembers() || workspaceMembers()!.length <= 0) {
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
																	const role = roles()?.roles.find((r) => r.id === roleId);
																	return {
																		id: roleId,
																		name: role?.name || roleId,
																	};
																}) || []
															}
															availableRoles={
																roles()?.roles.map((role) => ({
																	id: role.id,
																	name: role.name,
																})) || []
															}
															onSave={async (roleIds: string[]) => {
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
												<tr class="border border-border-color min-h-10 flex items-center justify-center w-full px-xl bg-secondary-light last-of-type:rounded-b-xs">
													<td class="flex items-center justify-center flex-2 gap-2">
														<Initials
															size="xs"
															firstName={member.userName.split(" ")[0]}
															lastName={member.userName.split(" ")[1]}
														/>
														{member.userName}
													</td>
													<td class="flex items-center justify-center flex-1">{memberRoleNames || "No roles"}</td>
													<td class="flex items-center justify-center flex-1">
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
																	onClick={() => {
																		setEditingMember({
																			userId: member.userId,
																			userName: member.userName,
																			roleIds: member.roleIds,
																		});
																	}}
																	class="text-gray-400 hover:bg-white/10 p-1 rounded transition-colors cursor-pointer"
																>
																	<FiEdit2 size={18} />
																</button>
																<button
																	onClick={(e) => {
																		e.stopPropagation();
																		setUserToDelete(member.userId);
																		setShouldDelete(true);
																	}}
																	class="text-red-500 hover:bg-white/10 p-1 rounded transition-colors cursor-pointer"
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
					loading={workspaceMembers.loading}
					showPageSizeSelector={false}
					showGoToPage={false}
				/>
			</PageContainerBody>
		</PageContainer>
	);
};

export default ManageWorkspace;
