import { ErrorBoundary, Show, Suspense } from "solid-js";
import { Title } from "@solidjs/meta";
import Alert from "~/components/alert";
import Button from "~/components/button";
import BgOnboard from "~/components/bg-onboard";
import Input, { InputType } from "~/components/input";
import { useToast } from "~/components/toast";
import { useCreateWorkspace } from "~/hooks/use-create-workspace";
import { ButtonVariant } from "~/utils/color";

/**
 * Shown in place of the dashboard when a logged-in (cloud) user has no
 * workspace yet. Replaces the old standalone `/onboard` route: there's no
 * redirect, so once the workspace is created the workspaces query refetches and
 * the workspaced layout swaps this screen out for the dashboard on its own.
 */
const CreateFirstWorkspace = () => {
	const toast = useToast();
	const { workspaceName, setWorkspaceName, nameError, setNameError, isLoading, submit } = useCreateWorkspace();

	return (
		<>
			<Title>Create Workspace | Patr</Title>
			<ErrorBoundary
				fallback={(err) => (
					<div class="min-h-full w-full flex items-center justify-center text-white p-8 text-center">
						<p>Something went wrong: {err.message}</p>
					</div>
				)}
			>
				<Suspense fallback={<div>Loading...</div>}>
					<main class="min-h-full w-full bg-secondary flex items-center justify-center p-4 relative overflow-hidden">
						<BgOnboard />

						<form
							noValidate
							onSubmit={async (e: SubmitEvent) => {
								e.preventDefault();
								await submit().catch(() =>
									toast("An unexpected error occurred. Please try again.", "error")
								);
							}}
							class="bg-secondary p-12 rounded-sm shadow-2xl w-full max-w-128 relative flex flex-col items-start justify-start gap-3 z-10 border border-secondary-medium"
						>
							<div class="text-left">
								<h1 class="text-2xl font-bold text-white">Create your workspace</h1>
								<p class="text-gray-400 text-sm">Set up a workspace to get started with Patr.</p>
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
					</main>
				</Suspense>
			</ErrorBoundary>
		</>
	);
};

export default CreateFirstWorkspace;
