import { useNavigate } from "@solidjs/router";
import { createMemo, createResource, ErrorBoundary, Suspense } from "solid-js";
import { ListDeploymentResponse } from "~/bindings";
import {
  PageContainer,
  PageContainerBody,
  PageContainerHead,
  Table,
} from "~/components";
import { useAuthState, useLastWorkspaceId } from "~/hooks/state-hooks";
import { doFetch } from "~/utils/do-fetch";

const ListDeploymentsPage = () => {
  const [authState] = useAuthState();
  const [workspaceId] = useLastWorkspaceId();
  const navigate = useNavigate();

  const fetchParams = createMemo(() => {
    return [authState(), workspaceId()] as const;
  });

  const [deployments] = createResource(fetchParams, async ([auth, wsId]) => {
    if (!wsId || !auth || auth.type !== "LoggedIn") {
      return { deployments: [] };
    }

    const response = await doFetch<ListDeploymentResponse>(
      `http://localhost:3001/api/workspace/${wsId}/deployment`,
      {
        method: "GET",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${auth.accessToken}`,
        },
      }
    );

    console.log("Fetched deployments:", response.data);

    // Fetch deployments logic goes here
    return { deployments: response.data.deployments };
  });

  return (
    <PageContainer>
      <PageContainerHead title="Deployments" subTitle="List of Deployments" />
      <PageContainerBody>
        <ErrorBoundary
          fallback={(err, reset) => (
            <div>
              <p>Error loading deployments: {err.message}</p>
              <button onClick={reset}>Retry</button>
            </div>
          )}
        >
          <Suspense fallback={<div>Loading deployments...</div>}>
            <Table
              column_grids={["flex-4", "flex-4", "flex-4", "flex-4"]}
              rows={deployments()?.deployments || []}
              headings={["ID", "Deployment Name", "Status", "Runner ID"]}
              renderRow={(item) => (
                <tr
                  onClick={() => {
                    navigate(`/deployments/${item.id}`);
                  }}
                  class="table-row"
                >
                  <td class="flex-4">{item.id}</td>
                  <td class="flex-4">{item.name}</td>
                  <td class="flex-4">{item.status}</td>
                  <td class="flex-4">{item.runner}</td>
                </tr>
              )}
            />
          </Suspense>
        </ErrorBoundary>
      </PageContainerBody>
    </PageContainer>
  );
};

export default ListDeploymentsPage;
