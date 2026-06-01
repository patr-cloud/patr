import { createFileRoute } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import { createEffect, createSignal, Show, Suspense } from "solid-js";
import { FiTrash2 } from "solid-icons/fi";

import {
	Button,
	ButtonVariant,
	DeleteModal,
	EmptyState,
	Link,
	LoadingSpinner,
	PageContainer,
	PageContainerBody,
	Pagination,
	Table,
	TableRow,
	TableCell,
	useToast,
} from "~/components";
import { Color } from "~/utils/color";
import { useNavigate } from "@tanstack/solid-router";
import { useAuthState, createPaginationState, useIsAllowed } from "~/hooks";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { Role, WithId } from "~/bindings";
import { httpRequest } from "~/utils/http-request";
import WorkspaceHeader from "~/routes/_logged-in/_workspaced/workspace/-components/workspace-header";
import { useRolesQuery, useWorkspaceInfoQuery } from "~/hooks/fetch";
import { useQueryClient } from "@tanstack/solid-query";
import { roleKeys } from "~/hooks/query-keys";

const RoleRow = (props: { role: WithId<Role>; onDeleted: () => void }) => {
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
		props.onDeleted();
	};

	return (
		<TableRow>
			<TableCell index={0}>
				<span class="truncate font-medium text-white">{props.role.name}</span>
			</TableCell>
			<TableCell index={1}>
				<span class={props.role.description ? "truncate" : "truncate text-grey italic"}>
					{props.role.description || "No description"}
				</span>
			</TableCell>
			<TableCell index={2} align="center">
				<Link
					href={`/workspace/roles/${props.role.id}`}
					buttonVariant={ButtonVariant.Plain}
					class="h-full flex items-center gap-2 cursor-pointer"
				>
					Manage Role
				</Link>
			</TableCell>
			<TableCell index={3} align="center">
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
			</TableCell>
		</TableRow>
	);
};

const ManageRoles = () => {
	const navigate = useNavigate();
	const isAllowedCreate = useIsAllowed("modifyRoles", "edit");
	const search = Route.useSearch();
	const pagination = createPaginationState({
		search: () => search(),
		navigate,
	});
	const [workspaceId] = useLastWorkspaceId();
	const queryClient = useQueryClient();

	const workspaceInfoQuery = useWorkspaceInfoQuery();
	const rolesQuery = useRolesQuery(
		() => search().page,
		() => search().count
	);

	createEffect(() => {
		const totalCount = rolesQuery.data?.totalCount;
		if (totalCount !== undefined) {
			pagination.setTotalCount(totalCount);
		}
	});

	const refetchRoles = () => {
		const wsId = workspaceId();
		if (wsId) {
			queryClient.invalidateQueries({ queryKey: roleKeys.all(wsId) });
		}
	};

	return (
		<>
			<Title>Roles | Patr</Title>
			<PageContainer>
				<WorkspaceHeader workspaceName={workspaceInfoQuery.data?.name} activeTab="roles" />
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
							when={(rolesQuery.data?.roles || []).length > 0}
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
								column_grids={["flex-3", "flex-5", "flex-3", "flex-1"]}
								headings={["Role Name", "Description", "Actions", ""]}
								rows={rolesQuery.data?.roles || []}
								renderRow={(role) => <RoleRow role={role} onDeleted={refetchRoles} />}
							/>
							<Pagination
								state={pagination}
								loading={rolesQuery.isFetching}
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
