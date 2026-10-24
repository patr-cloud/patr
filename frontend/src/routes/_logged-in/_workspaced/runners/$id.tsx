import { createFileRoute, useNavigate } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import { useQueryClient } from "@tanstack/solid-query";
import { createSignal, ErrorBoundary, Match, Show, Switch } from "solid-js";
import { DeleteRunnerResponse, RegenerateRunnerTokenResponse } from "~/bindings";
import {
	Button,
	ButtonVariant,
	CopyableField,
	DeleteModal,
	HeadTab,
	LoadingSpinner,
	Modal,
	ModalContainer,
	NoPermissionsPage,
	PageContainer,
	PageContainerBody,
	PageContainerHead,
	StatusChip,
	useToast,
} from "~/components";
import { createAuthenticatedAction } from "~/hooks";
import useIsAllowed, { useGetPermissions } from "~/hooks/is-allowed";
import { useRunnerInfoQuery } from "~/hooks/fetch";
import { runnerKeys } from "~/hooks/query-keys";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { Color } from "~/utils/color";
import { httpRequest } from "~/utils/http-request";
import RegenerateModal from "~/routes/_logged-in/profile/api-tokens/-components/regenerate-modal";
import RunnerDeployments from "./-components/deployments";
import RunnerMetrics from "./-components/metrics";
import RunnerLogs from "./-components/logs";
import RunnerInfo from "./-components/info";

const RunnerDetail = () => {
	const params = Route.useParams();
	const search = Route.useSearch();
	const tab = () => search().tab;

	const navigate = useNavigate();
	const toast = useToast();
	const queryClient = useQueryClient();
	const [workspaceId] = useLastWorkspaceId();
	const [isDeleteModalOpen, setIsDeleteModalOpen] = createSignal(false);
	const [isRegenerateModalOpen, setIsRegenerateModalOpen] = createSignal(false);
	const [newToken, setNewToken] = createSignal("");

	const isAllowedResource = useIsAllowed("runner", "view", params().id);
	const runnerPermissions = useGetPermissions("runner", () => params().id);

	const runnerQuery = useRunnerInfoQuery(() => params().id);

	const runner = () => runnerQuery.data?.runner;
	const reconnectCommand = () => `patr -w ${workspaceId()} runner setup reconnect --runner-id ${params().id}`;

	const { execute: deleteRunner, isLoading: isDeletingRunner } = createAuthenticatedAction(
		async ({ workspaceId: wsId }) => {
			const response = await httpRequest<DeleteRunnerResponse>(
				`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/runner/${params().id}`,
				{ method: "DELETE" }
			);

			if (!response.ok) {
				if (response.data.error === "resourceInUse") {
					toast("Cannot delete runner: it still has deployments", "error");
					return;
				}
				toast("Failed to delete runner", "error");
				return;
			}

			queryClient.invalidateQueries({ queryKey: runnerKeys.all(wsId) });
			toast("Runner deleted successfully", "success");
			navigate({ to: "/runners" });
		}
	);

	const { execute: regenerateToken, isLoading: isRegeneratingToken } = createAuthenticatedAction(
		async ({ workspaceId }) => {
			if (!runnerPermissions().regenerateToken) {
				toast("You do not have permission to regenerate this runner's token", "error");
				return;
			}

			const resp = await httpRequest<RegenerateRunnerTokenResponse>(
				`${import.meta.env.VITE_BASE_URL}/api/workspace/${workspaceId}/runner/${params().id}/token`,
				{ method: "POST" }
			);
			if (!resp.ok) {
				toast(resp.data.message || "Failed to regenerate the runner's token", "error");
				return;
			}

			setIsRegenerateModalOpen(false);
			setNewToken(resp.data.token);
			queryClient.invalidateQueries({ queryKey: runnerKeys.detail(workspaceId, params().id) });
		}
	);

	return (
		<>
			<Title>Runner Details | Patr</Title>
			<Show
				when={isAllowedResource()}
				fallback={
					<NoPermissionsPage
						title="Can't View Resource"
						message="You do not have permission to view this runner."
					/>
				}
			>
				<PageContainer>
					<ErrorBoundary
						fallback={(err, reset) => (
							<div>
								<p>Error loading runner info: {err.message}</p>
								<button onClick={reset}>Retry</button>
							</div>
						)}
					>
						<Show
							when={!runnerQuery.isPending}
							fallback={
								<div class="flex items-center justify-center gap-2 py-16 text-grey">
									<LoadingSpinner size={20} />
									<span class="text-sm">Loading runner...</span>
								</div>
							}
						>
							<PageContainerHead
								breadcrumbs={[
									{
										label: "Runners",
										url: "/runners",
									},
									{
										label: runner()?.name ?? "Loading...",
									},
								]}
								subText="View deployments, system metrics, and logs for this runner."
								class="justify-between items-center"
								actions={() => (
									<div class="flex items-center gap-sm">
										<Show when={runner()}>
											<StatusChip
												status={
													runner()!.connected
														? "connected"
														: runner()!.lastSeen
															? "unreachable"
															: "not set up"
												}
												size="md"
											/>
										</Show>
										<Show when={runner() && runnerPermissions().regenerateToken}>
											<RegenerateModal
												title="Regenerate Runner Token"
												message="The machine this runner is on disconnects within 30 seconds and stays offline until it's given the new token. Running deployments keep running and nothing redeploys, but if it uses a private tunnel, their URLs are down until it reconnects."
												resourceName={runner()?.name || ""}
												isOpen={isRegenerateModalOpen}
												setIsOpen={setIsRegenerateModalOpen}
												isLoading={isRegeneratingToken()}
												onClickRegenerate={(e) => {
													e.preventDefault();
													regenerateToken();
												}}
												renderTrigger={(open) => (
													<Button
														onClick={() => open(true)}
														variant={ButtonVariant.Outlined}
														color={Color.Error}
													>
														Regenerate Token
													</Button>
												)}
											/>
										</Show>
										<Show when={runnerPermissions().delete && runnerQuery.data?.runner.name}>
											<DeleteModal
												isLoading={isDeletingRunner()}
												title="Do You Really Want to Delete This Runner?"
												resourceName={runnerQuery.data!.runner.name}
												isOpen={isDeleteModalOpen}
												setIsOpen={setIsDeleteModalOpen}
												onClickDelete={(e) => {
													e.preventDefault();
													deleteRunner();
												}}
												renderTrigger={(open) => (
													<Button
														class="h-10"
														onClick={() => open?.(true)}
														variant={ButtonVariant.Outlined}
														color={Color.Error}
													>
														Delete
													</Button>
												)}
											/>
										</Show>
									</div>
								)}
								bottomContent={() => (
									<HeadTab
										tab={tab}
										tabItems={[
											{
												label: "Deployments",
												value: "deployments",
												onClick: (value) =>
													navigate({
														to: "/runners/$id",
														params: { id: params().id },
														search: { tab: value },
													}),
											},
											{
												label: "Info",
												value: "info",
												onClick: (value) =>
													navigate({
														to: "/runners/$id",
														params: { id: params().id },
														search: { tab: value },
													}),
											},
											{
												label: "Metrics",
												value: "metrics",
												onClick: (value) =>
													navigate({
														to: "/runners/$id",
														params: { id: params().id },
														search: { tab: value },
													}),
											},
											{
												label: "Logs",
												value: "logs",
												onClick: (value) =>
													navigate({
														to: "/runners/$id",
														params: { id: params().id },
														search: { tab: value },
													}),
											},
										]}
									/>
								)}
							/>

							<PageContainerBody class="flex flex-col justify-between gap-8">
								<Switch fallback={<div class="text-grey text-sm py-8 text-center">No such tab</div>}>
									<Match when={tab() === "deployments"}>
										<RunnerDeployments runnerId={params().id} />
									</Match>
									<Match when={tab() === "info"}>
										<RunnerInfo runnerId={params().id} reconnectCommand={reconnectCommand()} />
									</Match>
									<Match when={tab() === "metrics"}>
										<RunnerMetrics runnerId={params().id} />
									</Match>
									<Match when={tab() === "logs"}>
										<RunnerLogs runnerId={params().id} />
									</Match>
								</Switch>
							</PageContainerBody>
						</Show>
					</ErrorBoundary>

					<Modal
						isOpen={() => newToken() !== ""}
						renderTrigger={() => <></>}
						renderModalContent={() => (
							<ModalContainer closeFn={() => setNewToken("")} class="max-w-200">
								<h2 class="text-md mb-4 text-primary">New Runner Token</h2>
								<p class="mb-3 text-sm text-white">
									Copy this token now. You won't be able to see it again.
								</p>
								<CopyableField value={newToken()} innerClass="font-mono" />
								<p class="mt-6 mb-3 text-sm text-white">
									On the machine that should run this runner, run:
								</p>
								<CopyableField
									value={`${reconnectCommand()} --runner-token ${newToken()}`}
									innerClass="font-mono"
								/>
							</ModalContainer>
						)}
					/>
				</PageContainer>
			</Show>
		</>
	);
};

export const Route = createFileRoute("/_logged-in/_workspaced/runners/$id")({
	validateSearch: (search: Record<string, unknown>): { tab: string } => ({
		tab: (search.tab as string) || "deployments",
	}),
	component: RunnerDetail,
});
