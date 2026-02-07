import { createResource, createSignal, Show, Suspense } from "solid-js";
import { useNavigate, useParams } from "@solidjs/router";
import { Button, ButtonVariant, Link, PageContainer, PageContainerBody, Table, useToast } from "~/components";
import { FiTrash2 } from "solid-icons/fi";
import { useAuthState } from "~/hooks";
import { GetWorkspaceInfoResponse } from "~/bindings/GetWorkspaceInfoResponse";
import { ListAllRolesResponse } from "~/bindings/ListAllRolesResponse";
import { httpRequest } from "~/utils/http-request";
import WorkspaceHeader from "~/pages/workspace/workspace-header";
import { Color } from "~/utils/color";
import { GetRoleInfoResponse, Role, WithId } from "~/bindings";

const EditRole = (props: { role: WithId<Role> }) => {
	const params = useParams();
	const [authState] = useAuthState();
	const toast = useToast();

	const [roleInfo] = createResource(async () => {
		const auth = authState();
		if (!auth || auth.type !== "LoggedIn") {
			return;
		}

		const workspaceId = params.id;
		if (!workspaceId) {
			toast("Workspace ID is missing", "error");
			return;
		}

		const response = await httpRequest<GetRoleInfoResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${workspaceId}/rbac/role/${props.role.id}`,
			{
				method: "GET",
				headers: {
					"Content-Type": "application/json",
					Authorization: `Bearer ${auth.accessToken}`,
				},
			}
		);

		if (!response.ok) {
			console.error("Failed to fetch role info:", response.data.error);
			toast("Failed to fetch role info", "error");
			return;
		}

		return response.data;
	});

	return (
		<div class="text-white w-full">
			{Object.entries(roleInfo()?.permissions || {}).map(([permissionId, permissionType]) => {
				return (
					<div class="flex justify-between items-center border-b border-border-color py-2">
						<div>{permissionId}</div>
						<div>{permissionType?.permissionType}</div>
					</div>
				);
			})}
		</div>
	);
};

const RoleRow = (props: {
	refetch: (info?: unknown) => ListAllRolesResponse | Promise<ListAllRolesResponse | undefined> | null | undefined;
	role: WithId<Role>;
}) => {
	const params = useParams();
	const [authState] = useAuthState();
	const toast = useToast();
	const [showManageRole, setShowManageRole] = createSignal<boolean>(false);

	const onClickDelete = async (roleId: string) => {
		const auth = authState();
		if (!auth || auth.type !== "LoggedIn") {
			toast("You must be logged in to delete a role", "error");
			return;
		}

		const workspaceId = params.id;
		if (!workspaceId) {
			toast("Workspace ID is missing", "error");
			return;
		}

		const resp = await httpRequest(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${workspaceId}/rbac/role/${roleId}?removeUsers=false`,
			{
				method: "DELETE",
			}
		);

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
					href={`/workspaces/${params.id}/roles/${props.role.id}`}
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
	const params = useParams();
	const [authState] = useAuthState();
	const toast = useToast();
	const navigate = useNavigate();
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

	const [roles, { refetch: refetchRoles }] = createResource(resourceParamsWorkspace, async ([auth, id]) => {
		if (!auth || auth.type !== "LoggedIn" || id === "") {
			return { roles: [] };
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
			return { roles: [] };
		}
		return response.data;
	});

	return (
		<PageContainer>
			<WorkspaceHeader workspaceName={workspaceInfo()?.name} activeTab="roles" />
			<PageContainerBody class="flex flex-col justify-between gap-8">
				<div class="flex flex-col gap-6">
					<Suspense fallback={<div class="text-white">Loading roles...</div>}>
						<Show when={(roles()?.roles || []).length > 0} fallback={<div class="text-white">No roles found</div>}>
							<Table
								column_grids={["flex-1", "flex-2", "flex-1", "flex-[0.5]"]}
								headings={["Role Name", "Description", "Action", ""]}
								rows={roles()?.roles || []}
								renderRow={(role) => <RoleRow role={role} refetch={refetchRoles} />}
							/>
						</Show>
					</Suspense>
				</div>
			</PageContainerBody>
		</PageContainer>
	);
};

export default ManageRoles;
