import { createResource, Show, Suspense } from "solid-js";
import { useNavigate, useParams } from "@solidjs/router";
import {
  Button,
  ButtonVariant,
  PageContainer,
  PageContainerBody,
  Table,
  useToast,
} from "~/components";
import { FiPlus, FiTrash2 } from "solid-icons/fi";
import { useAuthState } from "~/hooks";
import { GetWorkspaceInfoResponse } from "~/bindings/GetWorkspaceInfoResponse";
import { ListAllRolesResponse } from "~/bindings/ListAllRolesResponse";
import { httpRequest } from "~/utils/http-request";
import WorkspaceHeader from "~/pages/workspace/workspace-header";

const ManageRoles = () => {
  const params = useParams();
  const [authState] = useAuthState();
  const toast = useToast();
  const navigate = useNavigate();
  const resourceParamsWorkspace = () => {
    return [authState(), params.id] as const;
  };

  const [workspaceInfo] = createResource(
    resourceParamsWorkspace,
    async ([auth, id]) => {
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
    }
  );

  const [roles] = createResource(
    resourceParamsWorkspace,
    async ([auth, id]) => {
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
    }
  );

  return (
    <PageContainer>
      <WorkspaceHeader
        workspaceName={workspaceInfo()?.name}
        activeTab="roles"
      />
      <PageContainerBody class="flex flex-col justify-between gap-8">
        <div class="flex flex-col gap-6">
          <Suspense fallback={<div class="text-white">Loading roles...</div>}>
            <Show
              when={roles()?.roles && roles()!.roles.length > 0}
              fallback={<div class="text-white">No roles found</div>}
            >
              <Table<Record<string, unknown>>
                column_grids={["flex-1", "flex-2", "flex-1", "flex-[0.5]"]}
                headings={["Role Name", "Description", "Action", ""]}
                rows={roles()!.roles as unknown as Record<string, unknown>[]}
                renderRow={(role, index) => (
                  <tr class="border border-border-color min-h-10 flex items-center justify-center w-full px-xl bg-secondary-light last-of-type:rounded-b-xs">
                    <td class="flex items-center justify-center flex-1">
                      {(role as any).name}
                    </td>
                    <td class="flex items-center justify-center flex-2">
                      {(role as any).description || "No description"}
                    </td>
                    <td class="flex items-center justify-center flex-1">
                      <span class="text-primary cursor-pointer hover:underline">
                        Manage Role
                      </span>
                    </td>
                    <td class="flex items-center justify-center flex-[0.5]">
                      <Button
                        onClick={() => {
                          console.log("Delete role:", (role as any).id);
                        }}
                        variant={ButtonVariant.Contained}
                        class="h-full flex items-center gap-2 bg-error"
                      >
                        <FiTrash2 size={16} />
                      </Button>
                    </td>
                  </tr>
                )}
              />
            </Show>
          </Suspense>
        </div>

        <div class="flex justify-end w-full items-center">
          <Button
            variant={ButtonVariant.Contained}
            class="bg-primary flex items-center gap-2"
            onClick={() => navigate(`/workspaces/${params.id}/roles/new`)}
          >
            <FiPlus size={16} />
          </Button>
        </div>
      </PageContainerBody>
    </PageContainer>
  );
};

export default ManageRoles;
