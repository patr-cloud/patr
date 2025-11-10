import { createResource, createSignal, Suspense } from "solid-js";
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
import { httpRequest } from "~/utils/http-request";
import { useAuthState } from "~/hooks";
import {
  CreateApiTokenRequest,
  CreateApiTokenResponse,
  ListUserWorkspacesResponse,
} from "~/bindings";
import { useToast } from "~/components/toast";

const CreateApiTokens = () => {
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
            auth.type === "LoggedIn" ? auth.accessToken : ""
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

  const [name, setName] = createSignal<string>("");
  const [fromDate, setFromDate] = createSignal<Date | null>(null);
  const [toDate, setToDate] = createSignal<Date | null>(null);
  const [workspaces, setWorkspaces] = createSignal<string[]>([]);

  const onSubmit = async (e: Event) => {
    e.preventDefault();
    const auth = authState();
    if (!auth || auth.type !== "LoggedIn") {
      console.error("User is not logged in");
      return;
    }

    console.log("Creating API Token with details:", {
      name: name(),
      fromDate: fromDate(),
      toDate: toDate(),
      workspaces: workspaces(),
    });

    const requestBody: CreateApiTokenRequest = {
      name: name(),
      created: new Date(),
      tokenNbf: fromDate() || undefined,
      tokenExp: toDate() || undefined,
      permissions: Object.fromEntries(
        workspaces().map((wsId) => [wsId, { type: "superAdmin" }])
      ),
    };
    const response = await httpRequest<CreateApiTokenResponse>(
      `${import.meta.env.VITE_BASE_URL}/api/user/api-token`,
      {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${
            auth.type === "LoggedIn" ? auth.accessToken : ""
          }`,
        },
        body: JSON.stringify(requestBody),
      }
    );

    console.log("API Token created successfully:", response.data);
  };

  return (
    <PageContainer>
      <PageContainerHead title="Create API Tokens" subTitle="subtitle" />
      <PageContainerBody class="flex flex-col justify-between gap-8">
        <form
          onSubmit={onSubmit}
          class="flex w-full flex-col justify-between gap-8 h-full flex-1"
        >
          <div class="flex flex-col gap-6 items-start w-full">
            <h1 class="text-md">Create API Tokens</h1>

            <div class="flex gap-8 items-center w-full">
              <InputLabel
                parentClass="flex-2"
                for="token-name"
                label="Token Name"
              />
              <Input
                value={name()}
                onInput={(e) => {
                  setName(e.currentTarget.value);
                }}
                class="flex-10"
                name="token-name"
                placeholder="Enter Token Name"
                type={InputType.Text}
              />
            </div>

            <div class="flex gap-8 items-center w-full">
              <InputLabel
                parentClass="flex-2"
                for="allowed-ips"
                label="Allowed IP(s)"
                comments="By default, all IP addresses will be allowed. Enter Comma Separated Values."
              />
              <Input
                class="flex-10"
                name="token-name"
                placeholder="Enter Comma Seperated IP(s)"
                type={InputType.Text}
              />
            </div>

            <div class="flex gap-8 items-center w-full">
              <InputLabel
                parentClass="flex-2"
                label="Token Validity"
                comments="By default, the token will be valid forever from the date created."
              />

              <div class="flex items-center flex-10 gap-4">
                <InputLabel
                  parentClass="flex-2"
                  for="token-validity-from"
                  label="Valid From"
                />
                <Input
                  class="flex-10"
                  value={
                    fromDate()
                      ? fromDate()?.toISOString().split("T")[0] ?? ""
                      : ""
                  }
                  onInput={(e) => {
                    setFromDate(e.currentTarget.valueAsDate);
                  }}
                  name="token-validity"
                  placeholder="Enter Token Validity in days"
                  type={InputType.Date}
                />

                <InputLabel
                  parentClass="flex-2 items-center"
                  for="token-validity-to"
                  label="to"
                />
                <Input
                  onInput={(e) => {
                    setToDate(e.currentTarget.valueAsDate);
                  }}
                  value={toDate() ? toDate()!.toISOString().split("T")[0] : ""}
                  class="flex-10"
                  name="token-validity"
                  placeholder="Enter Token Validity in days"
                  type={InputType.Date}
                />
              </div>
            </div>

            <div class="flex gap-8 items-center w-full">
              <InputLabel parentClass="flex-2" label="Workspace" />

              <div class="flex flex-10">
                <Suspense fallback={<div>Loading...</div>}>
                  {workspace() ? (
                    <ul>
                      {workspace()!.workspaces.map((ws) => {
                        return (
                          <li>
                            <label>{ws.name}</label>
                            <input
                              onInput={(e) => {
                                if (e.currentTarget.checked) {
                                  setWorkspaces((prev) => [...prev, ws.id]);
                                } else {
                                  setWorkspaces((prev) =>
                                    prev.filter((id) => id !== ws.id)
                                  );
                                }
                              }}
                              type="checkbox"
                              value={ws.id}
                            />
                          </li>
                        );
                      })}
                    </ul>
                  ) : (
                    <div>No workspaces found</div>
                  )}
                </Suspense>
              </div>
            </div>
          </div>

          <div class="flex justify-end">
            <Button type="submit" variant={ButtonVariant.Contained}>
              Create Token
            </Button>
          </div>
        </form>
      </PageContainerBody>
    </PageContainer>
  );
};
export default CreateApiTokens;
