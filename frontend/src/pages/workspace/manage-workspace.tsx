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
import { FiPlus } from "solid-icons/fi";
import { useAuthState } from "~/hooks";
import { GetWorkspaceInfoResponse } from "~/bindings/GetWorkspaceInfoResponse";
import { ListAllRolesResponse } from "~/bindings/ListAllRolesResponse";
import { ListUsersInWorkspaceResponse } from "~/bindings/ListUsersInWorkspaceResponse";
import { GetUserDetailsResponse } from "~/bindings/GetUserDetailsResponse";
import { UpdateUserRolesInWorkspaceRequest } from "~/bindings/UpdateUserRolesInWorkspaceRequest";
import { WithId } from "~/bindings/WithId";
import { BasicUserInfo } from "~/bindings/BasicUserInfo";
import { httpRequest } from "~/utils/http-request";
import WorkspaceHeader from "~/pages/workspace/workspace-header";
import { EventT } from "~/utils/types";

interface TeamMember {
  userId: string;
  userName: string;
  roleId: string;
  roleName: string;
}

const ManageWorkspace = () => {
  const params = useParams();
  const [authState] = useAuthState();
  const toast = useToast();
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
    }
  );

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
        console.error(
          "Failed to fetch workspace members:",
          response.data.error
        );
        toast("Failed to fetch workspace members", "error");
        return undefined;
      }
      // Fetch user details for each user ID
      const userDetailsPromises = Object.keys(response.data.users).map(
        async (userId) => {
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
            const firstName =
              user.firstName || (user as any).basic_user_info?.firstName || "";
            const lastName =
              user.lastName || (user as any).basic_user_info?.lastName || "";
            const username =
              user.username || (user as any).basic_user_info?.username || "";
            const id = user.id || (user as any).basic_user_info?.id || userId;

            return {
              userId: id,
              userName: `${firstName} ${lastName} (@${username})`,
              roleIds: roleIds,
            };
          }
          console.error(
            "Failed to fetch user details for",
            userId,
            ":",
            userResponse.data
          );
          return null;
        }
      );

      const userDetails = await Promise.all(userDetailsPromises);
      return userDetails.filter((user) => user !== null);
    }
  );

  // Separate state for input fields and added members
  const [selectedUser, setSelectedUser] =
    createSignal<WithId<BasicUserInfo> | null>(null);
  const [currentRoleId, setCurrentRoleId] = createSignal("");
  const [isSubmitting, setIsSubmitting] = createSignal(false);

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
        `${import.meta.env.VITE_BASE_URL}/api/workspace/${
          params.id
        }/rbac/user/${user.id}`,
        {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
            Authorization: `Bearer ${auth.accessToken}`,
          },
          body: JSON.stringify(requestBody),
        }
      );

      if (!response.ok) {
        console.error("Failed to add user:", response.data.error);
        toast(
          response.data.error || "Failed to add user to workspace",
          "error"
        );
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

  return (
    <PageContainer>
      <WorkspaceHeader
        workspaceName={workspaceInfo()?.name}
        activeTab="workspace"
      />
      <PageContainerBody class="flex flex-col justify-between gap-8">
        <div class="flex flex-col gap-6">
          <div class="flex flex-col gap-4">
            <form
              class="p-lg bg-secondary-light rounded-xs"
              onSubmit={handleAddMember}
            >
              <h1 class="text-lg mb-3">Create New Managed URL</h1>

              <div class="flex flex-col items-start justify-center gap-2 w-full">
                <div class="flex items-center justify-center gap-3 w-full">
                  <Show
                    when={authState() && authState()!.type === "LoggedIn"}
                    fallback={<div class="flex-2" />}
                  >
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

            <Suspense
              fallback={<div class="text-white">Loading members...</div>}
            >
              <Table
                column_grids={["flex-2", "flex-1"]}
                headings={["User", "Roles"]}
                rows={workspaceMembers() || []}
                renderRow={(member) => {
                  const memberRoleIds = member.roleIds;
                  const memberRoleNames = memberRoleIds
                    .map(
                      (roleId) =>
                        roles()?.roles.find((r) => r.id === roleId)?.name
                    )
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
                  return (
                    <tr class="border border-border-color min-h-10 flex items-center justify-center w-full px-xl bg-secondary-light last-of-type:rounded-b-xs">
                      <td class="flex items-center justify-center flex-2">
                        {member.userName}
                      </td>
                      <td class="flex items-center justify-center flex-1">
                        {memberRoleNames || "No roles"}
                      </td>
                    </tr>
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
