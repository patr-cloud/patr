import { useParams } from "@solidjs/router";
import { createMemo, createResource, ErrorBoundary, Suspense } from "solid-js";
import { GetDomainInfoInWorkspaceResponse } from "~/bindings";
import { PageContainer, PageContainerHead, useToast } from "~/components";
import { useAuthState } from "~/hooks";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { httpRequest } from "~/utils/http-request";

const DomainInfo = () => {
  const params = useParams();

  const [authState] = useAuthState();
  const [workspaceId] = useLastWorkspaceId();
  const toast = useToast();

  const resourceParams = createMemo(() => {
    return [authState(), workspaceId(), params.id] as const;
  });

  const [domainInfo] = createResource(
    resourceParams,
    async ([auth, wsId, domainId]) => {
      if (!wsId || !auth || auth.type !== "LoggedIn" || !domainId) {
        return;
      }

      // Fetch domain info logic goes here
      const resource = await httpRequest<GetDomainInfoInWorkspaceResponse>(
        `${
          import.meta.env.VITE_BASE_URL
        }/api/workspace/${wsId}/domain/${domainId}`,
        {
          method: "GET",
          headers: {
            "Content-Type": "application/json",
            Authorization: `Bearer ${auth.accessToken}`,
          },
        }
      );

      if (!resource.ok) {
        console.error("Failed to fetch domain info:", resource.data.error);
        toast("Failed to fetch domain info", "error");
        return;
      }

      console.log("Fetched domain info:", resource.data);
      return resource.data;
    }
  );

  return (
    <PageContainer>
      <ErrorBoundary
        fallback={(err, reset) => (
          <div>
            <p>Error loading runners: {err.message}</p>
            <button onClick={reset}>Retry</button>
          </div>
        )}
      >
        <Suspense fallback={<div>Loading...</div>}>
          <PageContainerHead title="Domain" subTitle={domainInfo()?.name} />
        </Suspense>
      </ErrorBoundary>
    </PageContainer>
  );
};

export default DomainInfo;
