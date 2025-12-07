import { createResource, ErrorBoundary, Suspense } from "solid-js";
import { ListUserWorkspacesResponse } from "~/bindings";
import {
  PageContainer,
  PageContainerBody,
  PageContainerHead,
  useToast,
} from "~/components";
import Table from "~/components/table";
import { httpRequest } from "~/utils/http-request";
import { useAuthState } from "~/hooks";

const ListWorkspaces = () => {
  const [authState, _] = useAuthState();
  const toast = useToast();

  const [workspace] = createResource(authState, async (auth) => {
    const response = await httpRequest<ListUserWorkspacesResponse>(
      `${import.meta.env.VITE_BASE_URL}/api/user/workspaces`,
      {
        method: "GET",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${
            auth.type === "LoggedIn" ? auth.accessToken : " "
          }`,
        },
      }
    );

    if (!response.ok) {
      console.error("Failed to fetch workspaces:", response.data.error);
      toast("Failed to fetch workspaces", "error");
      return { workspaces: [] };
    }

    return response.data;
  });

  return (
    <PageContainer>
      <PageContainerHead title="Workspaces" subTitle="All Workspaces" />
      <PageContainerBody class="flex flex-col justify-between gap-8">
        <ErrorBoundary
          fallback={(err, reset) => (
            <div>
              <p>Error loading workspaces: {err.message}</p>
              <button onClick={reset}>Retry</button>
            </div>
          )}
        >
          <Suspense fallback={<div>Loading...</div>}>
            <Table
              column_grids={["flex-1", "flex-1"]}
              headings={["Id", "Name"]}
              rows={workspace()?.workspaces || []}
              renderRow={(item) => (
                <tr class="table-row">
                  <td class="flex-1">{item.id}</td>
                  <td class="flex-1">{item.name}</td>
                </tr>
              )}
            />
          </Suspense>
        </ErrorBoundary>
      </PageContainerBody>
    </PageContainer>
  );
};

export default ListWorkspaces;
