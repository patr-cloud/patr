import { useParams } from "@solidjs/router";
import {
  createMemo,
  createResource,
  createSignal,
  ErrorBoundary,
  Suspense,
} from "solid-js";
import {
  GetDomainInfoInWorkspaceResponse,
  ListManagedURLResponse,
} from "~/bindings";
import {
  Button,
  ButtonVariant,
  Input,
  InputDropdown,
  PageContainer,
  PageContainerBody,
  PageContainerHead,
  useToast,
} from "~/components";
import { useAuthState } from "~/hooks";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { httpRequest } from "~/utils/http-request";
import { EventT } from "~/utils/types";

const DomainInfo = () => {
  const params = useParams();

  const [authState] = useAuthState();
  const [workspaceId] = useLastWorkspaceId();
  const toast = useToast();
  const [isVerifying, setIsVerifying] = createSignal(false);

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

  const resourceParamsManagedUrls = createMemo(() => {
    return [authState(), workspaceId(), params.id] as const;
  });
  const [managedUrls, { refetch: refetchManagedUrls }] = createResource(
    resourceParamsManagedUrls,
    async ([auth, wsId, domainId]) => {
      if (!wsId || !auth || auth.type !== "LoggedIn" || !domainId) {
        return;
      }

      // Fetch managed URLs logic goes here
      const resource = await httpRequest<ListManagedURLResponse>(
        `${
          import.meta.env.VITE_BASE_URL
        }/api/workspace/${wsId}/infrastructure/managed-url?domain_id=${domainId}`,
        {
          method: "GET",
          headers: {
            "Content-Type": "application/json",
            Authorization: `Bearer ${auth.accessToken}`,
          },
        }
      );

      if (!resource.ok) {
        console.error("Failed to fetch managed URLs:", resource.data.error);
        toast("Failed to fetch managed URLs", "error");
        return;
      }

      console.log("Fetched managed URLs:", resource.data);
      return resource.data;
    }
  );

  const onVerifyClick = async (e: EventT<MouseEvent, HTMLButtonElement>) => {
    // Verify domain logic goes here
    toast("Domain verified successfully", "success");

    setIsVerifying(true);
  };

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
          <PageContainerHead
            title="Domains"
            titleUrl="/domains"
            subTitle={domainInfo.latest?.name}
          >
            {!domainInfo.latest?.isVerified ? (
              <Button
                type="button"
                onClick={onVerifyClick}
                variant={ButtonVariant.Contained}
                disabled={isVerifying()}
              >
                {isVerifying() ? "Verifying..." : "Verify"}
              </Button>
            ) : null}
          </PageContainerHead>
          <PageContainerBody>
            <h1 class="text-md mb-2">
              Managed URLs For {domainInfo.latest?.name}
            </h1>

            <form class="mb-2" onSubmit={() => {}}>
              <div class="flex items-center justify-center gap-2 w-full">
                <Input class="flex-2" placeholder="Sub-domain" />
                <span class="h-full">.</span>
                <Input class="flex-4" placeholder="Domain" />
                <span>/</span>
                <Input class="flex-2" placeholder="Path" />
                <p class="mx-2">Will point to</p>
                <InputDropdown
                  onSelect={() => {}}
                  options={[
                    {
                      label: "Deployments",
                      value: "deployment",
                    },
                    {
                      label: "Redirection",
                      value: "redirection",
                    },
                    {
                      label: "Proxy",
                      value: "proxy",
                    },
                  ]}
                  class="flex-2 m-0"
                  placeholder="Type"
                />
                <Input class="flex-4" placeholder="Domain" />
              </div>
            </form>

            <div class="flex flex-col gap-2 items-start w-5/5 mt-4">
              {managedUrls.latest?.urls?.map((url) => (
                <div class="flex items-center justify-center gap-2 w-full">
                  <span class="h-full">{url.subDomain}</span>
                  <span class="h-full">.</span>
                  <span class="h-full">{domainInfo.latest?.name}</span>
                  <span class="h-full">/</span>
                  <span class="h-full">{url.path}</span>
                  <p class="mx-2">Points to</p>
                  {/* <span class="h-full">{url.urlType}</span>
                  <span class="h-full">{url.destination || "N/A"}</span> */}
                </div>
              ))}
            </div>
          </PageContainerBody>
        </Suspense>
      </ErrorBoundary>
    </PageContainer>
  );
};

export default DomainInfo;
