import { A } from "@solidjs/router";
import { createMemo, createResource, Suspense } from "solid-js";
import { ListApiTokensResponse } from "~/bindings";
import {
  PageContainer,
  PageContainerBody,
  PageContainerHead,
  Table,
  useToast,
} from "~/components";
import { useAuthState } from "~/hooks";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { httpRequest } from "~/utils/http-request";

const ListApiTokens = () => {
  const [authState] = useAuthState();
  const [workspaceId] = useLastWorkspaceId();
  const toast = useToast();

  const fetchParams = createMemo(() => {
    return [authState(), workspaceId()] as const;
  });

  const [apiTokens] = createResource(fetchParams, async ([auth, wsId]) => {
    if (!wsId || !auth || auth.type !== "LoggedIn") {
      return { tokens: [] };
    }

    const response = await httpRequest<ListApiTokensResponse>(
      `${import.meta.env.VITE_BASE_URL}/api/user/api-token`,
      {
        method: "GET",
      }
    );

    if (!response.ok) {
      console.error("Failed to fetch API Tokens:", response.data.error);
      toast("Failed to fetch API Tokens", "error");
      return { tokens: [] };
    }

    console.log("Fetched API Tokens:", response.data);

    // Fetch deployments logic goes here
    return { tokens: response.data.tokens };
  });

  return (
    <PageContainer>
      <PageContainerHead
        titleUrl="/profile"
        title="User"
        subTitle="API Tokens"
        actions={() => <A href="/profile/api-tokens/new">Create API Token</A>}
      />
      <PageContainerBody class="flex flex-col gap-8">
        <Suspense fallback={<div>Loading API Tokens...</div>}>
          <Table
            column_grids={["flex-4", "flex-4", "flex-4"]}
            headings={["Token Name", "Created", "Expiry"]}
            rows={apiTokens()?.tokens || []}
            renderRow={(token) => (
              <tr class="table-row">
                <td class="flex-4 flex items-center justify-center">
                  {token.name}
                </td>
                <td class="flex-4 flex items-center justify-center">
                  {/* @ts-expect-error - Date formatting will be fixed later */}
                  {token.created || "Unknown"}
                </td>
                <td class="flex-4 flex items-center justify-center">
                  {/* @ts-expect-error - Date formatting will be fixed later */}
                  {token.tokenExp ? token.tokenExp : "Never"}
                </td>
              </tr>
            )}
          />
        </Suspense>
      </PageContainerBody>
    </PageContainer>
  );
};

export default ListApiTokens;
