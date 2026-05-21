import { createFileRoute, useNavigate } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import { useQueryClient } from "@tanstack/solid-query";
import { createEffect, createSignal, ParentProps, Show, Suspense } from "solid-js";
import { CreateWorkspaceResponse } from "~/bindings";
import { Alert, BgOnboard, Button, Input, InputType, useToast } from "~/components";

import { useAuthState, useLastWorkspaceId } from "~/hooks/state-hooks";
import { useWorkspacesQuery } from "~/hooks/fetch";
import { workspacesKeys } from "~/hooks/query-keys";
import { ButtonVariant } from "~/utils/color";
import { cloudOnly } from "~/utils/env";
import { httpRequest } from "~/utils/http-request";
import { EventT } from "~/utils/types";

const WorkspaceOnboardPage = (_props: ParentProps) => {
	return <WorkspaceOnboard />;
};

const WorkspaceOnboard = () => {
	const [authState] = useAuthState();
	const [workspaceName, setWorkspaceName] = createSignal("");
	const [nameError, setNameError] = createSignal("");
	const toast = useToast();
	const [isLoading, setIsLoading] = createSignal(false);
	const navigate = useNavigate();

	const [, setWorkspaceId] = useLastWorkspaceId();
	const workspacesQuery = useWorkspacesQuery();
	const queryClient = useQueryClient();

	createEffect(() => {
		if ((workspacesQuery.data?.workspaces?.length || 0) > 0) {
			navigate({ to: "/", replace: true });
		}
	});

	const onCreateWorkspace = async (e: EventT<SubmitEvent, HTMLFormElement>) => {
		e.preventDefault();

		// Set loading synchronously up front so a second click that fires before
		// the first reaches its async hop sees the flag and bails. Setting it
		// only after the early-returns means two near-simultaneous clicks both
		// pass the `isLoading()` check.
		if (isLoading()) return;
		setIsLoading(true);

		const auth = authState();
		if (!auth || auth.type !== "LoggedIn") {
			toast("You must be logged in to create a workspace", "error");
			setIsLoading(false);
			return;
		}

		const name = workspaceName().trim();
		if (!name) {
			setNameError("Workspace name is required.");
			setIsLoading(false);
			return;
		}
		try {
			const response = await httpRequest<CreateWorkspaceResponse>(
				`${import.meta.env.VITE_BASE_URL}/api/workspace`,
				{
					method: "POST",
					body: JSON.stringify({ name }),
				}
			);

			if (!response.ok) {
				setNameError("Failed to create workspace. Please try a different name.");
				return;
			}

			toast("Workspace created successfully", "success");

			if (response.data.id) {
				setWorkspaceId(response.data.id);
				await queryClient.invalidateQueries({ queryKey: workspacesKeys.list() });
				navigate({ to: "/" });
			}
		} catch {
			toast("An unexpected error occurred. Please try again.", "error");
		} finally {
			setIsLoading(false);
		}
	};

	return (
		<>
			<Title>Create Workspace | Patr</Title>
			<Suspense fallback={<div>Loading...</div>}>
				<main class="min-h-screen w-full bg-secondary flex items-center justify-center p-4 relative overflow-hidden">
					<BgOnboard />

					<form
						noValidate
						onSubmit={onCreateWorkspace}
						class="bg-secondary p-12 rounded-sm shadow-2xl w-full max-w-128 relative flex flex-col items-start justify-start gap-3 z-10 border border-secondary-medium"
					>
						<div class="text-left">
							<h1 class="text-2xl font-bold text-white">Create Workspace</h1>
							<p class="text-gray-400 text-sm">Set up your workspace to get started with Patr.</p>
						</div>

						<div class="w-full">
							<div class="w-full mb-4">
								<Input
									type={InputType.Text}
									placeholder="Enter your workspace name"
									name="workspace-name"
									id="workspace-name"
									value={workspaceName}
									onInput={(e: Event) => {
										setWorkspaceName((e.currentTarget as HTMLInputElement).value);
										setNameError("");
									}}
									styleVariant="medium"
								/>
								<Show when={nameError()}>
									<div class="mt-1">
										<Alert message={nameError()} type="error" />
									</div>
								</Show>
							</div>

							<div class="flex items-center justify-end">
								<Button
									loading={isLoading()}
									loadingContent={() => <span>Creating...</span>}
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
						<p class="text-gray-500 text-xs">
							&copy; {new Date().getFullYear()} Patr. All rights reserved.
						</p>
					</div>
				</main>
			</Suspense>
		</>
	);
};

export const Route = createFileRoute("/_logged-in/_non-workspaced/onboard")(
	cloudOnly({
		component: WorkspaceOnboardPage,
	}),
);
