import { useNavigate } from "@solidjs/router";
import {
  createMemo,
  createResource,
  createSignal,
  ErrorBoundary,
  Suspense,
} from "solid-js";
import { FiCheck, FiCopy } from "solid-icons/fi";
import { ListDeploymentResponse } from "~/bindings";
import {
  PageContainer,
  PageContainerBody,
  PageContainerHead,
  Table,
} from "~/components";
import { useAuthState, useLastWorkspaceId } from "~/hooks/state-hooks";
import { doFetch } from "~/utils/do-fetch";

const CopyButton = (props: { text: string }) => {
  const [copied, setCopied] = createSignal(false);

  const handleCopy = async (e: MouseEvent) => {
    e.stopPropagation(); // Prevent row click navigation
    try {
      await navigator.clipboard.writeText(props.text);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch (error) {
      console.error("Failed to copy:", error);
    }
  };

  return (
    <button
      onClick={handleCopy}
      class="ml-2 p-1 rounded hover:bg-white/10 transition-colors"
      title={copied() ? "Copied!" : "Copy ID"}
    >
      {copied() ? (
        <FiCheck size={14} class="text-gray-400" />
      ) : (
        <FiCopy size={14} class="text-gray-400" />
      )}
    </button>
  );
};

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
      `${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/deployment`,
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
              headings={["ID", "Deployment Name", "Status", "Runner"]}
              renderRow={(item) => (
                <tr
                  onClick={() => {
                    navigate(`/deployments/${item.id}`);
                  }}
                  class="table-row"
                >
                  <td class="flex-4 flex items-center justify-center">
                    <span class="truncate">{item.id}</span>
                    <CopyButton text={item.id} />
                  </td>
                  <td class="flex-4 flex items-center justify-center">
                    {item.name}
                  </td>
                  <td class="flex-4 flex items-center justify-center">
                    {item.status}
                  </td>
                  <td class="flex-4 flex items-center justify-center">
                    {item.runner}
                  </td>
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
