import { createSignal } from "solid-js";
import { AddRunnerToWorkspaceResponse } from "~/bindings";
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
import { httpRequest } from "~/utils/http-request";

const CreateRunnerPage = () => {
  const [name, setName] = createSignal<string>("");
  const [authState] = useAuthState();
  const [workspaceId] = useLastWorkspaceId();

  const onSubmit = async (e: SubmitEvent) => {
    e.preventDefault();
    const auth = authState();
    const currentWorkspaceId = workspaceId();
    if (!auth || auth.type !== "LoggedIn" || !currentWorkspaceId) {
      console.error("User is not logged in");
      return;
    }

    console.log("Creating Runner with name:", name());
    const response = await httpRequest<AddRunnerToWorkspaceResponse>(
      `${
        import.meta.env.VITE_BASE_URL
      }/api/workspace/${currentWorkspaceId}/runner`,
      {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${auth.accessToken}`,
        },
        body: JSON.stringify({
          name: name(),
        }),
      }
    );

    setName("");
    console.log("Runner created successfully:", response.data);
  };

  return (
    <PageContainer>
      <PageContainerHead title="Runners" titleUrl="/runners" subTitle="New" />
      <PageContainerBody class="flex flex-col justify-between gap-8">
        <form
          onSubmit={onSubmit}
          class="flex flex-col gap-8 items-start w-full justify-between flex-1"
        >
          <div class="flex w-full flex-col justify-between gap-6 h-full flex-1">
            <div class="flex flex-col gap-6 items-start w-full">
              <h1 class="text-md">Create Runner</h1>

              <div class="flex gap-8 items-center w-full">
                <InputLabel
                  parentClass="flex-2"
                  for="runner-name"
                  label="Runner Name"
                />
                <Input
                  value={name()}
                  onInput={(e) => {
                    setName(e.currentTarget.value);
                  }}
                  class="flex-10"
                  name="runner-name"
                  placeholder="Enter Runner Name"
                  type={InputType.Text}
                />
              </div>
            </div>
          </div>

          <div class="w-full flex justify-end">
            <Button variant={ButtonVariant.Contained} type="submit">
              Create Runner
            </Button>
          </div>
        </form>
      </PageContainerBody>
    </PageContainer>
  );
};

export default CreateRunnerPage;
