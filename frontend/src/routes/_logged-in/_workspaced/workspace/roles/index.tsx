import { createFileRoute } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import { createResource, Show, Suspense } from "solid-js";

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
import { FiTrash2 } from "solid-icons/fi";
import { useNavigate } from "@tanstack/solid-router";
import { useAuthState, createPaginationState } from "~/hooks";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { GetWorkspaceInfoResponse } from "~/bindings/GetWorkspaceInfoResponse";
import { ListAllRolesResponse } from "~/bindings/ListAllRolesResponse";
import { httpRequest } from "~/utils/http-request";
import WorkspaceHeader from "~/routes/_logged-in/_workspaced/workspace/-components/workspace-header";
import { Color } from "~/utils/color";
import { Role, WithId } from "~/bindings";

const RoleRow = (props: {
	refetch: (info?: unknown) => ListAllRolesResponse | Promise<ListAllRolesResponse | undefined> | null | undefined;
	role: WithId<Role>;
}) => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();
	const toast = useToast();

	const onClickDelete = async (roleId: string) => {
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

		const resp = await httpRequest(`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/rbac/role/${roleId}`, {
			method: "DELETE",
		});

		if (!resp.ok) {
			console.error("Failed to delete role:", resp.data.error);
			toast(resp.data.message, "error");
			return;
		}

		toast("Role deleted successfully", "success");
		props.refetch();
	};

	return (
		<tr class="border border-border-color min-h-10 flex items-center justify-center w-full px-xl bg-secondary-light last-of-type:rounded-b-xs">
			<td class="flex items-center justify-center flex-1">{props.role.name}</td>
			<td class="flex items-center justify-center flex-2">{props.role.description || "No description"}</td>
			<td class="flex items-center justify-center flex-1">
				<Link
					external
					href={`/workspace/roles/${props.role.id}`}
					buttonVariant={ButtonVariant.Plain}
					class="h-full flex items-center gap-2 cursor-pointer"
				>
					Manage Role
				</Link>
			</td>
			<td class="flex items-center justify-center flex-[0.5]">
				<Button
					onClick={(e) => {
						e.preventDefault();
						onClickDelete(props.role.id);
					}}
					color={Color.Error}
					variant={ButtonVariant.Plain}
					class="h-full flex items-center gap-2 cursor-pointer"
				>
					<FiTrash2 size={16} />
				</Button>
			</td>
		</tr>
	);
};

const ManageRoles = () => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();
	const toast = useToast();
	const navigate = useNavigate();
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
			console.error("Failed to fetch workspace info:", response.data.error);
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
			console.error("Failed to fetch roles:", response.data.error);
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
				<PageContainerBody class="flex flex-col justify-between gap-8">
					<div class="flex flex-col gap-6 flex-1">
						<Suspense fallback={<div class="text-white">Loading roles...</div>}>
							<Show
								when={(roles()?.roles || []).length > 0}
								fallback={<EmptyState title="No Roles Created" />}
							>
								<Table
									column_grids={["flex-1", "flex-2", "flex-1", "flex-[0.5]"]}
									headings={["Role Name", "Description", "Action", ""]}
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
					</div>
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
