import { useParams } from "@solidjs/router";
import { createMemo, createResource, ErrorBoundary, Suspense } from "solid-js";
import { GetDeploymentInfoResponse, GetRunnerInfoResponse } from "~/bindings";
import {
  Button,
  ButtonVariant,
  Input,
  InputLabel,
  InputType,
  PageContainer,
  PageContainerBody,
  PageContainerHead,
} from "~/components";
import { useAuthState } from "~/hooks";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { doFetch } from "~/utils/do-fetch";

const DeploymentInfo = () => {
  const params = useParams();

  const [authState] = useAuthState();
  const [workspaceId] = useLastWorkspaceId();

  const resourceParamsDeployment = createMemo(() => {
    return [authState(), workspaceId(), params.id] as const;
  });

  const [deploymentInfo] = createResource(
    resourceParamsDeployment,
    async ([auth, wsId, id]) => {
      if (!wsId || !auth || auth.type !== "LoggedIn" || id === "") {
        return undefined;
      }
      const response = await doFetch<GetDeploymentInfoResponse>(
        `http://localhost:3001/api/workspace/${wsId}/deployment/${id}`,
        {
          method: "GET",
          headers: {
            "Content-Type": "application/json",
            Authorization: `Bearer ${auth.accessToken}`,
          },
        }
      );
      console.log("Fetched deployment info:", response.data);
      return response.data;
    }
  );

  const resourceParamsRunner = createMemo(() => {
    return [authState(), workspaceId(), deploymentInfo()?.runner] as const;
  });

  const [runnerInfo] = createResource(
    resourceParamsRunner,
    async ([auth, wsId, runnerId]) => {
      if (
        !wsId ||
        !auth ||
        auth.type !== "LoggedIn" ||
        !runnerId ||
        runnerId === ""
      ) {
        return undefined;
      }
      const response = await doFetch<GetRunnerInfoResponse>(
        `http://localhost:3001/api/workspace/${wsId}/runner/${runnerId}`,
        {
          method: "GET",
          headers: {
            "Content-Type": "application/json",
            Authorization: `Bearer ${auth.accessToken}`,
          },
        }
      );
      console.log("Fetched runner info:", response.data);
      return response.data.runner;
    }
  );

  const onClickStart = async (
    e: MouseEvent & { currentTarget: HTMLButtonElement }
  ) => {
    e.preventDefault();
    const auth = authState();
    const currentWorkspace = workspaceId();
    const deployment = deploymentInfo();

    if (!auth || auth.type !== "LoggedIn" || !currentWorkspace || !deployment) {
      console.error("User not logged in or workspace ID missing");
      return;
    }

    console.log("Start deployment clicked");
    const resp = await doFetch(
      `http://localhost:3001/api/workspace/${workspaceId()}/deployment/${
        deployment.id
      }/start`,
      {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${auth.accessToken}`,
        },
      }
    );
    console.log("Start deployment response:", resp);
  };

  const onClickStop = async (
    e: MouseEvent & { currentTarget: HTMLButtonElement }
  ) => {
    e.preventDefault();

    const auth = authState();
    const currentWorkspace = workspaceId();
    const deployment = deploymentInfo();

    if (!auth || auth.type !== "LoggedIn" || !currentWorkspace || !deployment) {
      console.error("User not logged in or workspace ID missing");
      return;
    }

    console.log("Stop deployment clicked");
    const resp = await doFetch(
      `http://localhost:3001/api/workspace/${workspaceId()}/deployment/${
        deployment.id
      }/stop`,
      {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${auth.accessToken}`,
        },
      }
    );
    console.log("Stop deployment response:", resp);
  };

  const Cta = () => {
    switch (deploymentInfo()?.status) {
      case "running":
        return (
          <Button
            onClick={onClickStop}
            class="h-10"
            variant={ButtonVariant.Outlined}
          >
            STOP
          </Button>
        );

      case "deploying":
        return <span>Deploying...</span>;
      case "errored":
        return <span>Error occurred</span>;
      case "unreachable":
        return <span>Unreachable</span>;
      case "stopped":
        return (
          <Button
            onClick={onClickStart}
            class="h-10"
            variant={ButtonVariant.Contained}
          >
            START
          </Button>
        );
      default:
        return <span>where status?</span>;
    }
  };

  return (
    <PageContainer>
      <ErrorBoundary
        fallback={(err, reset) => (
          <div>
            <p>Error loading deployment info: {err.message}</p>
            <button onClick={reset}>Retry</button>
          </div>
        )}
      >
        <Suspense fallback={<div>Loading deployment info...</div>}>
          <PageContainerHead
            title="Deployment"
            class="justify-between items-center"
            subTitle={`${
              deploymentInfo
                ? deploymentInfo?.loading
                  ? "Loading..."
                  : deploymentInfo()?.name
                : "No deployment found"
            }`}
          >
            {Cta()}
          </PageContainerHead>
          <PageContainerBody class="flex flex-col justify-between gap-8">
            <form class="flex flex-col gap-6 justify-between w-full flex-1">
              <div class="flex flex-col gap-4 items-start w-full">
                <div class="flex gap-8 items-center w-full">
                  <InputLabel
                    parentClass="flex-2"
                    for="deployment-id"
                    label="ID"
                  />
                  <Input
                    value={deploymentInfo()?.id}
                    disabled={true}
                    class="flex-10"
                    name="deployment-id"
                    placeholder="Deployment ID"
                    type={InputType.Text}
                  />
                </div>

                <div class="flex gap-8 items-center w-full">
                  <InputLabel
                    parentClass="flex-2"
                    for="deployment-name"
                    label="Name"
                  />
                  <Input
                    value={deploymentInfo()?.name}
                    disabled={true}
                    class="flex-10"
                    name="deployment-name"
                    placeholder="Deployment Name"
                    type={InputType.Text}
                  />
                </div>

                <div class="flex gap-8 items-center w-full">
                  <InputLabel
                    parentClass="flex-2"
                    for="deployment-runner"
                    label="Runner"
                  />
                  <Input
                    value={runnerInfo()?.name}
                    disabled={true}
                    class="flex-10"
                    name="deployment-runner"
                    placeholder="Runner"
                    type={InputType.Text}
                  />
                </div>

                <div class="flex gap-8 items-center w-full">
                  <InputLabel
                    parentClass="flex-2"
                    for="deployment-registry"
                    label="Registry"
                  />
                  <div class="flex-10 flex items-center gap-4 w-full">
                    <Input
                      value={deploymentInfo()?.registry ?? ""}
                      disabled={true}
                      class="flex-4"
                      name="deployment-registry"
                      placeholder="Select Registry"
                    />

                    <Input
                      class="flex-6"
                      disabled={true}
                      placeholder="Image Name"
                      type={InputType.Text}
                      value={(() => {
                        const info = deploymentInfo();
                        if (!info) return "";
                        if (info.registry === "registry.patr.cloud") {
                          return (info as any).repositoryId ?? "";
                        }
                        return (info as any).imageName ?? "";
                      })()}
                    />

                    <Input
                      class="flex-2"
                      disabled={true}
                      placeholder="Image Tag"
                      type={InputType.Text}
                      value={deploymentInfo()?.imageTag ?? "N/A"}
                    />
                  </div>
                </div>
              </div>
            </form>
          </PageContainerBody>
        </Suspense>
      </ErrorBoundary>
    </PageContainer>
  );
};

export default DeploymentInfo;
