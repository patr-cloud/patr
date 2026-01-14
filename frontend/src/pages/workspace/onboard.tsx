import { Navigate } from "@solidjs/router";
import { createSignal, ParentProps, Suspense } from "solid-js";
import { CreateWorkspaceResponse } from "~/bindings";
import { BgOnboard, useToast } from "~/components";
import Button from "~/components/button";
import Input, { InputType } from "~/components/input";
import { useAuthState, useLastWorkspaceId } from "~/hooks/state-hooks";
import useFetchWorkspaces from "~/hooks/use-fetch/use-fetch-wokrspaces";
import { ButtonVariant } from "~/utils/color";
import { httpRequest } from "~/utils/http-request";
import { EventT } from "~/utils/types";

const WorkspaceOnboardPage = (props: ParentProps<{}>) => {
  return <WorkspaceOnboard />;
};

const WorkspaceOnboard = () => {
  const [authState] = useAuthState();
  const [workspaceName, setWorkspaceName] = createSignal("");
  const toast = useToast();

  const [, setWorkspaceId] = useLastWorkspaceId();
  const [workspaces] = useFetchWorkspaces();

  if ((workspaces()?.workspaces?.length || 0) > 0) {
    return <Navigate href="/" />;
  }

  const onCreateWorkspace = async (e: EventT<SubmitEvent, HTMLFormElement>) => {
    e.preventDefault();

    const auth = authState();
    if (!auth || auth.type !== "LoggedIn") {
      toast("You must be logged in to create a workspace", "error");
      return;
    }

    const requestBody = {
      name: workspaceName(),
    };

    const response = await httpRequest<CreateWorkspaceResponse>(
      `${import.meta.env.VITE_BASE_URL}/api/workspace`,
      {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify(requestBody),
      }
    );

    if (!response.ok) {
      console.error("Failed to create workspace:", response.data.error);
      toast("Failed to create workspace", "error");
      return;
    }

    toast("Workspace created successfully", "success");
    setWorkspaceName("");

    if (response.data.id) {
      setWorkspaceId(response.data.id);
    }
  };

  return (
    <Suspense fallback={<div>Loading...</div>}>
      <main class="min-h-screen w-full bg-secondary flex items-center justify-center p-4 relative overflow-hidden">
        <BgOnboard />

        <form
          onSubmit={onCreateWorkspace}
          class="bg-secondary-dark p-12 rounded-xs shadow-2xl w-full max-w-[520px] relative flex flex-col items-start justify-start gap-3 z-10 border border-secondary-medium"
        >
          <div class="text-left">
            <h1 class="text-xl font-bold text-primary">Create Workspace</h1>
            <p class="text-gray-400 text-sm">
              Set up your workspace to get started with Patr.
            </p>
          </div>

          <div class="w-full">
            <div class="w-full mb-4">
              <Input
                type={InputType.Text}
                placeholder="Enter your workspace name"
                value={workspaceName}
                onInput={(e: Event) =>
                  setWorkspaceName((e.currentTarget as HTMLInputElement).value)
                }
                styleVariant="medium"
              />
            </div>

            <div class="flex items-center justify-end">
              <Button
                variant={ButtonVariant.Contained}
                class="w-full py-4 text-base font-semibold"
                type="submit"
              >
                Create Workspace
              </Button>
            </div>
          </div>
        </form>

        {/* Footer */}
        <div class="absolute bottom-6 left-0 right-0 text-center">
          <p class="text-gray-500 text-xs">© 2025 Patr. All rights reserved.</p>
        </div>
      </main>
    </Suspense>
  );
};

export default WorkspaceOnboardPage;
