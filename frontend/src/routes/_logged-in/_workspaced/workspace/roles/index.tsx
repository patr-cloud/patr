import { createFileRoute } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import { createEffect, createSignal, Show, Suspense } from "solid-js";
import { FiChevronDown, FiChevronUp, FiTrash2 } from "solid-icons/fi";

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
	useToast,
} from "~/components";
import RoleUsersChips from "./-components/role-users-chips";
import { Color } from "~/utils/color";
import { useNavigate } from "@tanstack/solid-router";
import { useAuthState, createPaginationState, useIsAllowed } from "~/hooks";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { WithId, Role } from "~/bindings";
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
	const [expanded, setExpanded] = createSignal(false);

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
		<tr
			role="row"
			class="flex flex-col w-full border border-border-color bg-secondary-light last-of-type:rounded-b-xs hover:bg-secondary-medium"
		>
			{/* Inner flex ratios below must stay in sync with the parent Table's column_grids. */}
			<td role="cell" class="flex items-center justify-center min-h-10 w-full px-md md:px-xl">
				<div class="flex-3 flex items-center justify-start min-w-0">
					<span class="truncate font-medium text-white">{props.role.name}</span>
				</div>
				<div class="flex-5 flex items-center justify-start min-w-0">
					<span class={props.role.description ? "truncate" : "truncate text-grey italic"}>
						{props.role.description || "No description"}
					</span>
				</div>
				<div class="flex-2 flex items-center justify-center min-w-0">
					<Link
						href={`/workspace/roles/${props.role.id}`}
						buttonVariant={ButtonVariant.Plain}
						class="h-full flex items-center gap-2 cursor-pointer"
					>
						Manage Role
					</Link>
				</div>
				<div class="flex-2 flex items-center justify-center min-w-0">
					<Button
						variant={ButtonVariant.Plain}
						aria-label={expanded() ? "Hide users" : "See users"}
						aria-expanded={expanded()}
						onClick={() => setExpanded(!expanded())}
						class="flex items-center gap-1 cursor-pointer"
					>
						<span>{expanded() ? "Hide users" : "See users"}</span>
						{expanded() ? <FiChevronUp size={14} /> : <FiChevronDown size={14} />}
					</Button>
				</div>
				<div class="flex-1 flex items-center justify-center min-w-0">
					{/* Built-in roles ship with the workspace and can't be deleted. */}
					<Show when={!props.role.isImmutable} fallback={<span class="text-grey text-xs">Built-in</span>}>
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
					</Show>
				</div>
			</td>
			<Show when={expanded()}>
				<td role="cell" class="w-full px-md md:px-xl py-sm border-t border-border-color/40">
					<RoleUsersChips roleId={props.role.id} />
				</td>
			</Show>
		</tr>
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
								column_grids={["flex-3", "flex-5", "flex-2", "flex-2", "flex-1"]}
								headings={["Role Name", "Description", "Actions", "", ""]}
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
