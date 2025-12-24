import { useParams } from "@solidjs/router";
import {
  createMemo,
  createResource,
  createSignal,
  ErrorBoundary,
  Suspense,
} from "solid-js";
import {
  CreateManagedURLRequest,
  CreateManagedURLResponse,
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
import DeploymentOption from "./deployment-option";

type urlTypeT = "proxyUrl" | "redirect" | "proxyDeployment" | "proxyStaticSite";

const DomainInfo = () => {
  const params = useParams();

  const [authState] = useAuthState();
  const [workspaceId] = useLastWorkspaceId();
  const toast = useToast();
  const [isVerifying, setIsVerifying] = createSignal(false);

  const [subDomain, setSubDomain] = createSignal("");
  const [path, setPath] = createSignal("");
  const [urlType, setUrlType] = createSignal<urlTypeT | null>(null);
  const [target, setTarget] = createSignal<string | null>(null);
  const [deploymentPort, setDeploymentPort] = createSignal<number | null>(null);

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

  const onSubmitCreateManagedUrl = async (
    e: EventT<SubmitEvent, HTMLFormElement>
  ) => {
    e.preventDefault();
    // Create managed URL logic goes here

    const auth = authState();
    const wsId = workspaceId();
    const domainId = params.id;

    if (!domainId || !wsId || !auth || auth.type !== "LoggedIn") {
      toast("Domain ID is missing", "error");
      return;
    }

    const urlTypeVal = urlType();
    const targetVal = target();
    if (!urlTypeVal || !subDomain() || !targetVal) {
      toast("Please fill in all required fields", "error");
      return;
    }

    const requestBody: CreateManagedURLRequest = {
      domainId,
      subDomain: subDomain(),
      path: path(),
      urlType: {
        type: "proxyDeployment",
        deploymentId: targetVal,
        port: deploymentPort() || 80,
      },
    };
    const response = await httpRequest<CreateManagedURLResponse>(
      `${
        import.meta.env.VITE_BASE_URL
      }/api/workspace/${wsId}/infrastructure/managed-url`,
      {
        method: "POST",
        body: JSON.stringify(requestBody),
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${auth.accessToken}`,
        },
      }
    );

    if (!response.ok) {
      console.error("Failed to create managed URL:", response.data.error);
      toast("Failed to create managed URL", "error");
      return;
    }

    toast("Managed URL created successfully", "success");

    refetchManagedUrls();
  };

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
        <Suspense fallback={<div class="text-red-500">Loading...</div>}>
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

            <form class="mb-2" onSubmit={onSubmitCreateManagedUrl}>
              <div class="flex items-center justify-center gap-2 w-full">
                <Input
                  onInput={(e) => setSubDomain(e.currentTarget.value)}
                  value={subDomain()}
                  class="flex-2"
                  placeholder="Sub-domain"
                />
                <span class="h-full">.</span>
                <Input
                  disabled={true}
                  value={domainInfo.latest?.name}
                  class="flex-2"
                  placeholder="Domain"
                />
                <span>/</span>
                <Input
                  onInput={(e) => setPath(e.currentTarget.value)}
                  value={path()}
                  class="flex-2"
                  placeholder="Path"
                />
                <p class="mx-2">Will point to</p>
                <InputDropdown
                  onSelect={(value) => setUrlType(value as urlTypeT)}
                  value={urlType() || undefined}
                  options={[
                    {
                      label: "Deployments",
                      value: "proxyDeployment",
                    },
                  ]}
                  class="flex-2 m-0"
                  placeholder="Type"
                />
                {urlType() === "proxyDeployment" && (
                  <DeploymentOption
                    deployment={target()}
                    onSelectDeployment={(value) => setTarget(value)}
                    port={deploymentPort() || 80}
                    onPortChange={(port) => setDeploymentPort(port)}
                  />
                )}
                <Button variant={ButtonVariant.Contained}>Create</Button>
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
                  {/* <span class="h-full">{url.urlType}</span> */}
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
