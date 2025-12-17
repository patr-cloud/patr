import { useParams } from "@solidjs/router";
import {
  createMemo,
  createResource,
  createSignal,
  ErrorBoundary,
  Suspense,
} from "solid-js";
import {
  GetDeploymentInfoResponse,
  GetRunnerInfoResponse,
  UpdateDeploymentResponse,
  ListRunnersForWorkspaceResponse,
} from "~/bindings";
import {
  Button,
  ButtonVariant,
  Input,
  InputDropdown,
  InputLabel,
  InputType,
  PageContainer,
  PageContainerBody,
  PageContainerHead,
  useToast,
} from "~/components";
import { useAuthState } from "~/hooks";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { httpRequest } from "~/utils/http-request";
import { EventT } from "~/utils/types";
import EnvInput from "./env-input";
import PortInput from "./port";
import { FiChevronDown } from "solid-icons/fi";

const DeploymentInfo = () => {
  const params = useParams();

  const [authState] = useAuthState();
  const [workspaceId] = useLastWorkspaceId();
  const toast = useToast();

  const resourceParamsDeployment = createMemo(() => {
    return [authState(), workspaceId(), params.id] as const;
  });

  const [hasUpdated, setHasUpdated] = createSignal(false);

  const [
    deploymentInfo,
    { mutate: mutateDeploymentInfo, refetch: refetchDeploymentInfo },
  ] = createResource(resourceParamsDeployment, async ([auth, wsId, id]) => {
    if (!wsId || !auth || auth.type !== "LoggedIn" || id === "") {
      return undefined;
    }
    const response = await httpRequest<GetDeploymentInfoResponse>(
      `${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/deployment/${id}`,
      {
        method: "GET",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${auth.accessToken}`,
        },
      }
    );
    console.log("Fetched deployment info:", response.data);
    if (!response.ok) {
      console.error("Failed to fetch deployment info:", response.data.error);
      toast("Failed to fetch deployment info", "error");
      return undefined;
    }

    return response.data;
  });

  const onSubmitUpdate = async (e: EventT<SubmitEvent, HTMLFormElement>) => {
    e.preventDefault();
    console.log("Update deployment form submitted");
    const auth = authState();
    if (!auth || auth.type !== "LoggedIn") {
      console.error("User not logged in");
      toast("User not logged in", "error");
      return;
    }

    const response = await httpRequest<UpdateDeploymentResponse>(
      `${
        import.meta.env.VITE_BASE_URL
      }/api/workspace/${workspaceId()}/deployment/${deploymentInfo()?.id}`,
      {
        method: "PATCH",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${auth.accessToken}`,
        },
        body: JSON.stringify({
          ...deploymentInfo(),
        }),
      }
    );

    if (!response.ok) {
      console.error("Failed to update deployment:", response.data.error);
      toast("Failed to update deployment", "error");
      refetchDeploymentInfo();
      return;
    }

    console.log("Deployment updated successfully:", response.data);
    toast("Deployment updated successfully", "success");
    refetchDeploymentInfo();
  };

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
      const response = await httpRequest<GetRunnerInfoResponse>(
        `${
          import.meta.env.VITE_BASE_URL
        }/api/workspace/${wsId}/runner/${runnerId}`,
        {
          method: "GET",
          headers: {
            "Content-Type": "application/json",
            Authorization: `Bearer ${auth.accessToken}`,
          },
        }
      );
      console.log("Fetched runner info:", response.data);

      if (!response.ok) {
        console.error("Failed to fetch runner info:", response.data.error);
        toast("Failed to fetch runner info", "error");
        return undefined;
      }

      return response.data.runner;
    }
  );

  const resourceParamsRunnerList = createMemo(() => {
    return [authState(), workspaceId()] as const;
  });

  const [runnerList] = createResource(
    resourceParamsRunnerList,
    async ([auth, wsId]) => {
      if (!wsId || !auth || auth.type !== "LoggedIn") {
        return { runners: [] };
      }
      const response = await httpRequest<ListRunnersForWorkspaceResponse>(
        `${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/runner`,
        {
          method: "GET",
          headers: {
            "Content-Type": "application/json",
            Authorization: `Bearer ${auth.accessToken}`,
          },
        }
      );

      if (!response.ok) {
        console.error("Failed to fetch runner list:", response.data.error);
        toast("Failed to fetch runner list", "error");
        return { runners: [] };
      }

      console.log("Fetched runner list:", response.data);
      return response.data;
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
    const resp = await httpRequest(
      `${
        import.meta.env.VITE_BASE_URL
      }/api/workspace/${workspaceId()}/deployment/${deployment.id}/start`,
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
    const resp = await httpRequest(
      `${
        import.meta.env.VITE_BASE_URL
      }/api/workspace/${workspaceId()}/deployment/${deployment.id}/stop`,
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

  const onClickDelete = async (
    e: MouseEvent & {
      currentTarget: HTMLButtonElement;
    }
  ) => {
    e.preventDefault();

    const auth = authState();
    const currentWorkspace = workspaceId();
    const deployment = deploymentInfo();

    if (!auth || auth.type !== "LoggedIn" || !currentWorkspace || !deployment) {
      console.error("User not logged in or workspace ID missing");
      return;
    }

    console.log("Delete deployment clicked");
    const resp = await httpRequest(
      `${
        import.meta.env.VITE_BASE_URL
      }/api/workspace/${workspaceId()}/deployment/${deployment.id}`,
      {
        method: "DELETE",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${auth.accessToken}`,
        },
      }
    );
    console.log("Delete deployment response:", resp);
  };

  const imageName = createMemo((): string => {
    const info = deploymentInfo();
    if (!info) return "";
    if (info.registry === "registry.patr.cloud") {
      return "repositoryId" in info
        ? String((info as any).repositoryId ?? "")
        : "";
    }
    return "imageName" in info ? String((info as any).imageName ?? "") : "";
  });

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
        return <span class="text-white">Deploying...</span>;
      case "errored":
        return <span class="text-white">Error occurred</span>;
      case "unreachable":
        return <span class="text-white">Unreachable</span>;
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
      <PageContainerHead
        title="Deployments"
        titleUrl="/deployments"
        class="justify-between items-center"
        subTitle={
          <Suspense fallback={<div>loading...</div>}>
            {deploymentInfo()?.name || "No Deployment Found"}
          </Suspense>
        }
      >
        <div class="flex items-center justify-end gap-8">
          <Suspense fallback={<div>Loading actions...</div>}>
            {Cta()}
            <Button onClick={onClickDelete} variant={ButtonVariant.Contained}>
              DELETE
            </Button>
          </Suspense>
        </div>
      </PageContainerHead>
      <PageContainerBody class="flex flex-col justify-between gap-8">
        <ErrorBoundary
          fallback={(err, reset) => (
            <div>
              <p>Error loading deployment info: {err.message}</p>
              <button onClick={reset}>Retry</button>
            </div>
          )}
        >
          <Suspense fallback={<div>Loading deployment info...</div>}>
            <form
              onSubmit={onSubmitUpdate}
              class="flex flex-col gap-6 justify-between w-full flex-1"
            >
              <div class="flex flex-col gap-4 items-start w-full">
                <div class="flex gap-8 items-center w-full">
                  <InputLabel
                    parentClass="flex-2"
                    for="deployment-id"
                    label="ID"
                  />
                  <Input
                    value={deploymentInfo.latest?.id}
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
                    value={deploymentInfo.latest?.name}
                    onInput={(e) => {
                      setHasUpdated(true);
                      mutateDeploymentInfo((prev) => {
                        return prev
                          ? {
                              ...prev,
                              name: e.currentTarget.value,
                            }
                          : undefined;
                      });
                    }}
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

                  <InputDropdown
                    options={
                      runnerList.latest?.runners.map((runner) => ({
                        value: runner.id,
                        label: runner.name,
                      })) ?? []
                    }
                    endIcon={() => (
                      <button>
                        <FiChevronDown size={16} />
                      </button>
                    )}
                    value={deploymentInfo.latest?.runner ?? ""}
                    onSelect={(runnerId) => {
                      setHasUpdated(true);
                      mutateDeploymentInfo((prev) => {
                        return prev
                          ? {
                              ...prev,
                              runner: runnerId,
                            }
                          : undefined;
                      });
                    }}
                    class="flex-10"
                    name="deployment-runner"
                    placeholder="Select Runner"
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
                      value={deploymentInfo.latest?.registry ?? ""}
                      disabled={true}
                      class="flex-4"
                      name="deployment-registry"
                      placeholder="Select Registry"
                    />

                    <Input
                      class="flex-6"
                      placeholder="Image Name"
                      type={InputType.Text}
                      onInput={(e) => {
                        setHasUpdated(true);
                        mutateDeploymentInfo((prev) => {
                          return prev
                            ? {
                                ...prev,
                                imageName: e.currentTarget.value,
                              }
                            : undefined;
                        });
                      }}
                      value={(() => {
                        const info = deploymentInfo.latest;
                        if (!info) return "";
                        if (info.registry === "registry.patr.cloud") {
                          return "repositoryId" in info
                            ? (info.repositoryId as string)
                            : "";
                        }
                        return "imageName" in info ? info.imageName : "";
                      })()}
                    />

                    <Input
                      class="flex-2"
                      placeholder="Image Tag"
                      type={InputType.Text}
                      value={deploymentInfo.latest?.imageTag ?? "N/A"}
                      onInput={(e) => {
                        setHasUpdated(true);
                        mutateDeploymentInfo((prev) => {
                          return prev
                            ? {
                                ...prev,
                                imageTag: e.currentTarget.value,
                              }
                            : undefined;
                        });
                      }}
                    />
                  </div>
                </div>

                <EnvInput
                  envList={Object.entries(
                    deploymentInfo.latest?.environmentVariables || {}
                  ).map(([key, value]) => ({ key, value }))}
                  onAdd={(key, value) => {
                    setHasUpdated(true);
                    mutateDeploymentInfo((prev) => {
                      return prev
                        ? {
                            ...prev,
                            environmentVariables: {
                              ...prev.environmentVariables,
                              [key]: value,
                            },
                          }
                        : undefined;
                    });
                  }}
                  onDelete={(key) => {
                    setHasUpdated(true);
                    mutateDeploymentInfo((prev) => {
                      if (!prev) return undefined;
                      const newEnv = { ...prev.environmentVariables };
                      delete newEnv[key];
                      return {
                        ...prev,
                        environmentVariables: newEnv,
                      };
                    });
                  }}
                />

                <PortInput
                  onAdd={(key, value) => {
                    setHasUpdated(true);
                    console.log(key, value);
                    mutateDeploymentInfo((prev) => {
                      return prev
                        ? {
                            ...prev,
                            ports: {
                              ...prev.ports,
                              [Number(key)]: value,
                            },
                          }
                        : undefined;
                    });
                  }}
                  onDelete={(key) => {
                    setHasUpdated(true);
                    mutateDeploymentInfo((prev) => {
                      if (!prev) return undefined;
                      const newPorts = { ...prev.ports };
                      delete newPorts[Number(key)];
                      return {
                        ...prev,
                        ports: newPorts,
                      };
                    });
                  }}
                  portList={deploymentInfo.latest?.ports || {}}
                />
              </div>

              <div class="w-full flex justify-end items-center">
                <Button type="submit" variant="contained">
                  UPDATE
                </Button>
              </div>
            </form>
          </Suspense>
        </ErrorBoundary>
      </PageContainerBody>
    </PageContainer>
  );
};

export default DeploymentInfo;
