import { createFileRoute } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import { createResource, createSignal, Show, Suspense } from "solid-js";
import { FiTrash2 } from "solid-icons/fi";

import {
	Button,
	ButtonVariant,
	EmptyState,
	Link,
	PageContainer,
	PageContainerBody,
	Pagination,
	Table,
	useToast,
} from "~/components";
import DeleteModal from "~/components/modal/delete-resource-modal";
import { LoadingSpinner } from "~/components/loading-spinner";
import { Color } from "~/utils/color";
import { useNavigate } from "@tanstack/solid-router";
import { useAuthState, createPaginationState, useIsAllowed } from "~/hooks";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { GetWorkspaceInfoResponse } from "~/bindings/GetWorkspaceInfoResponse";
import { ListAllRolesResponse } from "~/bindings/ListAllRolesResponse";
import { httpRequest } from "~/utils/http-request";
import WorkspaceHeader from "~/routes/_logged-in/_workspaced/workspace/-components/workspace-header";
import { Role, WithId } from "~/bindings";

const RoleRow = (props: { refetch: () => void; role: WithId<Role> }) => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();
	const toast = useToast();
	const [deleteOpen, setDeleteOpen] = createSignal(false);

	const onClickDelete = async () => {
		const auth = authState();
		if (!auth || auth.type !== "LoggedIn") {
			toast("You must be logged in to delete a role", "error");
			return;
		}

		const wsId = workspaceId();
		if (!wsId) {
			toast("Workspace ID is missing", "error");
			return;
		}

		const resp = await httpRequest(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/rbac/role/${props.role.id}`,
			{
				method: "DELETE",
			}
		);

		if (!resp.ok) {
			toast(resp.data.message || "Failed to delete role", "error");
			return;
		}

		toast("Role deleted successfully", "success");
		props.refetch();
	};

	return (
		<tr role="row" class="table-row">
			<td role="cell" class="flex-3 flex items-center justify-start min-w-0">
				<span class="truncate font-medium text-white">{props.role.name}</span>
			</td>
			<td role="cell" class="flex-4 flex items-center justify-start min-w-0">
				<span class={props.role.description ? "truncate" : "truncate text-grey italic"}>
					{props.role.description || "No description"}
				</span>
			</td>
			<td role="cell" class="flex-3 flex items-center justify-center min-w-0">
				<Link
					href={`/workspace/roles/${props.role.id}`}
					buttonVariant={ButtonVariant.Plain}
					class="h-full flex items-center gap-2 cursor-pointer"
				>
					Manage Role
				</Link>
			</td>
			<td role="cell" class="flex-[0.5] flex items-center justify-center min-w-0">
				<DeleteModal
					title={`Delete Role "${props.role.name}"`}
					resourceName={props.role.name}
					onClickDelete={onClickDelete}
					isOpen={deleteOpen}
					setIsOpen={setDeleteOpen}
					renderTrigger={(open) => {
						return (
							<Button
								variant={ButtonVariant.Plain}
								color={Color.Error}
								aria-label="Delete role"
								onClick={(e) => {
									e.stopPropagation();
									open?.(true);
								}}
								class="p-1"
							>
								<FiTrash2 size={16} />
							</Button>
						);
					}}
				/>
			</td>
		</tr>
	);
};

const ManageRoles = () => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();
	const toast = useToast();
	const navigate = useNavigate();
	const isAllowedCreate = useIsAllowed("modifyRoles", "edit");
	const search = Route.useSearch();
	const pagination = createPaginationState({
		search: () => search(),
		navigate,
	});
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
			}
		);
		if (!response.ok) {
			toast("Failed to fetch workspace info", "error");
			return undefined;
		}
		return response.data;
	});

	const rolesFetchParams = () => {
		return [authState(), workspaceId(), pagination.page(), pagination.count()] as const;
	};

	const [roles, { refetch: refetchRoles }] = createResource(rolesFetchParams, async ([auth, id, page, count]) => {
		if (!auth || auth.type !== "LoggedIn" || id === "") {
			return { roles: [] };
		}
		const response = await httpRequest<ListAllRolesResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${id}/rbac/role?page=${page}&count=${count}`,
			{
				method: "GET",
			}
		);
		if (!response.ok) {
			toast("Failed to fetch roles", "error");
			return { roles: [] };
		}
		pagination.setTotalCount(Number(response.headers.get("x-total-count") ?? 0));
		return response.data;
	});

	return (
		<>
			<Title>Roles | Patr</Title>
			<PageContainer>
				<WorkspaceHeader workspaceName={workspaceInfo()?.name} activeTab="roles" />
				<PageContainerBody class="flex flex-col justify-between">
					<Suspense
						fallback={
							<div class="flex items-center justify-center gap-2 py-16 text-grey">
								<LoadingSpinner size={20} />
								<span class="text-sm">Loading roles...</span>
							</div>
						}
					>
						<Show
							when={(roles()?.roles || []).length > 0}
							fallback={
								<EmptyState
									title="No Roles Created"
									description={
										isAllowedCreate() ? "Create roles to manage team permissions." : undefined
									}
									action={
										isAllowedCreate() ? (
											<Link
												href="/workspace/roles/new"
												buttonVariant={ButtonVariant.Outlined}
												external={false}
											>
												Create Role
											</Link>
										) : undefined
									}
								/>
							}
						>
							<Table
								column_grids={["flex-3", "flex-4", "flex-3", "flex-[0.5]"]}
								headings={["Role Name", "Description", "Actions", ""]}
								rows={roles()?.roles || []}
								renderRow={(role) => <RoleRow role={role} refetch={refetchRoles} />}
							/>
							<Pagination
								state={pagination}
								loading={roles.loading}
								showPageSizeSelector={false}
								showGoToPage={false}
							/>
						</Show>
					</Suspense>
				</PageContainerBody>
			</PageContainer>
		</>
	);
};

export const Route = createFileRoute("/_logged-in/_workspaced/workspace/roles/")({
	validateSearch: (search: Record<string, unknown>): { page?: string; count?: string } => ({
		page: (search.page as string) || undefined,
		count: (search.count as string) || undefined,
	}),
	component: ManageRoles,
});
