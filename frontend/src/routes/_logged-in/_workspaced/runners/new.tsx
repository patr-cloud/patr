import { createFileRoute, useNavigate } from "@tanstack/solid-router";
import { useQueryClient } from "@tanstack/solid-query";
import { Title } from "@solidjs/meta";
import { createSignal, Show } from "solid-js";
import { CreateRunnerRequest, CreateRunnerResponse } from "~/bindings";
import {
	Alert,
	Button,
	ButtonVariant,
	CopyableField,
	CopyableFieldVariant,
	Input,
	InputType,
	Label,
	PageContainer,
	PageContainerBody,
	PageContainerHead,
	UnsavedChangesGuard,
} from "~/components";
import { createFormAction } from "~/hooks";
import { runnerKeys } from "~/hooks/query-keys";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { IS_CLOUD } from "~/utils/env";
import { httpRequest } from "~/utils/http-request";
import { RESOURCE_NAME_REGEX } from "~/utils/validation";

const INSTALL_COMMAND =
	"curl -fsSL https://raw.githubusercontent.com/patr-cloud/patr/develop/assets/cli/install.sh | sh";

const CreateRunnerPage = () => {
	const [workspaceId] = useLastWorkspaceId();
	const navigate = useNavigate();
	const queryClient = useQueryClient();

	const [name, setName] = createSignal("");
	const [nameError, setNameError] = createSignal("");
	const [created, setCreated] = createSignal<{ workspaceId: string; id: string; name: string; token: string }>();
	const [isLeaving, setIsLeaving] = createSignal(false);
	const [tokenCopied, setTokenCopied] = createSignal(false);

	const { onSubmit, isLoading } = createFormAction(async ({ workspaceId: wsId }) => {
		const runnerName = name().trim();

		if (!runnerName) {
			setNameError("Runner name is required.");
			return;
		}

		if (!RESOURCE_NAME_REGEX.test(runnerName)) {
			setNameError(
				runnerName.length < 2 || runnerName.length > 255
					? "Runner name must be 2 to 255 characters long."
					: "Runner name can only contain letters, numbers, spaces, dots (.), hyphens (-) and underscores (_)."
			);
			return;
		}

		const requestBody: CreateRunnerRequest = { name: runnerName };
		const response = await httpRequest<CreateRunnerResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/runner`,
			{
				method: "POST",
				body: JSON.stringify(requestBody),
			}
		);

		if (!response.ok) {
			setNameError(
				response.data.error === "resourceAlreadyExists"
					? `A runner named "${runnerName}" already exists`
					: "Failed to create the runner. Please try again."
			);
			return;
		}

		queryClient.invalidateQueries({ queryKey: runnerKeys.list(wsId) });
		setCreated({ workspaceId: wsId, id: response.data.id, name: runnerName, token: response.data.token });
	});

	return (
		<>
			<Title>New Runner | Patr</Title>
			<PageContainer>
				<PageContainerHead
					subText="Runners execute deployments on your machines or clusters"
					breadcrumbs={[{ label: "Runners", url: "/runners" }, { label: "Add" }]}
				/>
				<PageContainerBody class="w-full">
					{IS_CLOUD && (
						<Show
							when={created()}
							fallback={
								<div class="flex flex-col gap-8 w-full">
									<p class="text-grey text-sm">
										Name the runner, then set it up on the machine that will run your deployments.
										The machine needs Linux with systemd, and Docker.
									</p>
									<form noValidate onSubmit={onSubmit} class="flex flex-col gap-8 w-full">
										<div class="flex gap-8 items-start w-full">
											<Label parentClass="flex-2 pt-2.5" for="runner-name" label="Runner Name" />
											<div class="flex-10 flex flex-col">
												<Input
													id="runner-name"
													name="runner-name"
													placeholder="Enter Runner Name"
													type={InputType.Text}
													value={name()}
													onInput={(e) => {
														setName(e.currentTarget.value);
														setNameError("");
													}}
												/>
												<Show when={nameError()}>
													<div class="mt-1">
														<Alert message={nameError()} type="error" />
													</div>
												</Show>
											</div>
										</div>

										<div class="w-full flex justify-end">
											<Button
												loading={isLoading}
												loadingContent={() => <span>Creating Runner...</span>}
												variant={ButtonVariant.Contained}
												type="submit"
											>
												Create Runner
											</Button>
										</div>
									</form>
									<div class="flex flex-wrap items-center gap-x-2 gap-y-1 text-grey text-xs">
										<span>Or from a logged-in Patr CLI on the machine:</span>
										<CopyableField
											variant={CopyableFieldVariant.Text}
											value="patr login"
											innerClass="text-white"
										/>
										<span>then</span>
										<CopyableField
											variant={CopyableFieldVariant.Text}
											value={`patr -w ${workspaceId() ?? "<workspace-id>"} runner setup new`}
											innerClass="text-white"
										/>
									</div>
								</div>
							}
						>
							{(runner) => (
								<div class="flex flex-col gap-6 w-full max-w-200">
									<Alert
										type="warning"
										message="Copy the runner's token now. It won't be shown again."
									/>
									<p class="text-grey text-sm">
										<span class="text-white">{runner().name}</span>&nbsp;is added. To connect it,
										run these on the machine that will run your deployments. The machine needs Linux
										with systemd, and Docker. It doesn't need to be logged in to Patr.
									</p>

									<div>
										<p class="text-gray-300 text-sm mb-2">Runner token</p>
										<CopyableField
											value={runner().token}
											innerClass="font-mono"
											onCopy={() => setTokenCopied(true)}
										/>
									</div>

									<div>
										<p class="text-gray-300 text-sm mb-2">1. Install the Patr CLI</p>
										<CopyableField value={INSTALL_COMMAND} innerClass="font-mono" />
										<p class="text-grey text-xs mt-2">
											Already installed? Run&nbsp;
											<code class="text-white font-log">patr upgrade</code>
											&nbsp;instead.
										</p>
									</div>

									<div>
										<p class="text-gray-300 text-sm mb-2">2. Connect the runner</p>
										<CopyableField
											value={`patr -w ${runner().workspaceId} runner setup reconnect --runner-id ${runner().id} --runner-token ${runner().token}`}
											innerClass="font-mono"
											onCopy={() => setTokenCopied(true)}
										/>
										<p class="text-grey text-xs mt-2">
											It asks for a few Docker settings, then saves the runner's config on the
											machine.
										</p>
									</div>

									<div>
										<p class="text-gray-300 text-sm mb-2">3. Start the runner as a service</p>
										<CopyableField value="patr runner service install" innerClass="font-mono" />
										<p class="text-grey text-xs mt-2">
											Run it as your normal user, not with sudo. It asks for your password when it
											needs to. The runner shows as connected once the service starts.
										</p>
									</div>

									<div class="w-full flex justify-end">
										<Button
											variant={ButtonVariant.Contained}
											onClick={() => {
												setIsLeaving(true);
												navigate({
													to: "/runners/$id",
													params: { id: runner().id },
													search: { tab: "metrics" },
												});
											}}
										>
											Go to Runner
										</Button>
									</div>
								</div>
							)}
						</Show>
					)}
					{!IS_CLOUD && (
						<p class="text-grey text-sm">
							Adding runners from the Patr CLI is only available on Patr Cloud for now. Ask your
							administrator how to add a runner to this instance.
						</p>
					)}
					<UnsavedChangesGuard
						when={() => !!created() && !isLeaving() && !tokenCopied()}
						title="Leave without the runner's token?"
						message="This runner's token won't be shown again. If you haven't copied it, you'll need to regenerate it from the runner's page."
					/>
				</PageContainerBody>
			</PageContainer>
		</>
	);
};

export const Route = createFileRoute("/_logged-in/_workspaced/runners/new")({
	component: CreateRunnerPage,
});
