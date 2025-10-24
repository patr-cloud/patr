import { createResource, ErrorBoundary, Suspense } from "solid-js";
import { ListUserWorkspacesResponse } from "~/bindings";
import {
  PageContainer,
  PageContainerBody,
  PageContainerHead,
} from "~/components";
import Table, { TableRow } from "~/components/table";
import { doFetch } from "~/utils/do-fetch";
import { useAuthState } from "~/utils/state";

const ListWorkspaces = () => {
  const [authState, _] = useAuthState();
  const [workspace] = createResource<ListUserWorkspacesResponse>(async () => {
    const auth = authState();
    const response = await doFetch<ListUserWorkspacesResponse>(
      "http://localhost:3001/api/user/workspaces",
      {
        method: "GET",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${
            auth.type === "LoggedIn" ? auth.accessToken : ""
          }`,
        },
      }
    );
    return response.data;
  });

  return (
    <PageContainer>
      <PageContainerHead title="Workspaces" subTitle="list" />
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
              column_grids={["flex-1", "flex-1", "flex-1"]}
              headings={["Name", "Description", "Actions"]}
              rows={workspace()?.workspaces || []}
              renderRow={(item) => (
                <TableRow
                  item={item}
                  column_classes={["flex-4", "flex-4", "flex-4"]}
                />
              )}
            />
          </Suspense>
        </ErrorBoundary>
      </PageContainerBody>
    </PageContainer>
  );
};

export default ListWorkspaces;
