import { createResource, createSignal, Show, Suspense } from "solid-js";
import { useParams } from "@solidjs/router";
import {
	Button,
	ButtonVariant,
	InputDropdown,
	PageContainer,
	PageContainerBody,
	Table,
	useToast,
	UserSearchInput,
} from "~/components";
import { FiEdit2, FiPlus, FiTrash } from "solid-icons/fi";
import { useAuthState } from "~/hooks";
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
import { EventT } from "~/utils/types";
import { EditRoles } from "~/pages/workspace/edit-roles";

const ManageWorkspace = () => {
	const params = useParams();
	const [authState] = useAuthState();
	const toast = useToast();
	const resourceParamsWorkspace = () => {
		return [authState(), params.id] as const;
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

	const [workspaceMembers, { refetch: refetchMembers }] = createResource(
		resourceParamsWorkspace,
		async ([auth, id]) => {
			if (!auth || auth.type !== "LoggedIn" || id === "") {
				return;
			}
			const response = await httpRequest<ListUsersInWorkspaceResponse>(
				`${import.meta.env.VITE_BASE_URL}/api/workspace/${id}/rbac/user`,
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
	const onDelete = async (e: EventT<MouseEvent, HTMLButtonElement>) => {
		e.stopPropagation();
		const wsId = params.id;
		const userId = userToDelete();
		const auth = authState();

		if (!userId) {
			toast("No user selected for deletion", "error");
			return;
		}

		if (!auth || auth.type !== "LoggedIn") {
			toast("Authentication required", "error");
			return;
		}

		try {
			const response = await httpRequest<RemoveUserFromWorkspaceResponse>(
				`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/rbac/user/${userId}`,
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
		} catch (error) {
			console.error("Error deleting user:", error);
			toast("An error occurred while removing the user", "error");
		}
	};
	// Separate state for input fields and added members
	const [selectedUser, setSelectedUser] = createSignal<WithId<BasicUserInfo> | null>(null);
	const [currentRoleId, setCurrentRoleId] = createSignal("");
	const [isSubmitting, setIsSubmitting] = createSignal(false);
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

	const handleAddMember = async (e: EventT<SubmitEvent, HTMLFormElement>) => {
		e.preventDefault();

		const user = selectedUser();
		const roleId = currentRoleId().trim();
		const auth = authState();

		if (!user || !roleId) {
			toast("Please select a user and role", "error");
			return;
		}

		if (!auth || auth.type !== "LoggedIn") {
			toast("You must be logged in", "error");
			return;
		}

		setIsSubmitting(true);

		try {
			const requestBody: UpdateUserRolesInWorkspaceRequest = {
				roles: [roleId],
			};

			const response = await httpRequest(
				`${import.meta.env.VITE_BASE_URL}/api/workspace/${params.id}/rbac/user/${user.id}`,
				{
					method: "POST",
					headers: {
						"Content-Type": "application/json",
					},
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
		} catch (error) {
			console.error("Error adding user:", error);
			toast("An error occurred while adding the user", "error");
		} finally {
			setIsSubmitting(false);
		}
	};

	const handleSaveRoles = async (roleIds: string[]) => {
		setEditingMember(null);
		refetchMembers();
	};

	return (
		<PageContainer>
			<WorkspaceHeader workspaceName={workspaceInfo()?.name} activeTab="workspace" />
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
												Loading...
											</tr>
										);
									}

									if (!workspaceMembers() || workspaceMembers()!.length <= 0) {
										return (
											<tr class="border border-border-color min-h-10 flex items-center justify-center w-full px-xl bg-secondary-light last-of-type:rounded-b-xs">
												No members found.
											</tr>
										);
									}

									const isEditing = editingMember()?.userId === member.userId;

									return (
										<>
											{isEditing ? (
												<tr class="table-row">
													<td class="w-full" colspan={3}>
														<EditRoles
															userName={editingMember()!.userName}
															userId={editingMember()!.userId}
															workspaceId={params.id || ""}
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
															onSave={handleSaveRoles}
															onClose={() => {
																setEditingMember(null);
															}}
														/>
													</td>
												</tr>
											) : (
												<tr class="border border-border-color min-h-10 flex items-center justify-center w-full px-xl bg-secondary-light last-of-type:rounded-b-xs">
													<td class="flex items-center justify-center flex-2">{member.userName}</td>
													<td class="flex items-center justify-center flex-1">{memberRoleNames || "No roles"}</td>
													<td class="flex items-center justify-center flex-1">
														{shouldDelete() && userToDelete() === member.userId ? (
															<>
																<div class="flex gap-2">
																	<button class="text-red-500" onClick={onDelete}>
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
			</PageContainerBody>
		</PageContainer>
	);
};

export default ManageWorkspace;
