import { createQuery } from "@tanstack/solid-query";
import { createMemo, createSignal, ErrorBoundary, Match, Show, Switch } from "solid-js";
import { GetRunnerLinkResponse } from "~/bindings";
import { LoadingSpinner } from "~/components";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { useWorkspacesQuery } from "~/hooks/fetch";
import { httpRequest } from "~/utils/http-request";
import { ApprovedState } from "./-approved-state";
import { LinkUnavailable } from "./-link-unavailable";
import { MachineDetails } from "./-machine-details";
import { ModeChoice } from "./-mode-choice";
import { NewRunnerForm } from "./-new-runner-form";
import { ReconnectForm } from "./-reconnect-form";

type Mode = "choice" | "new" | "reconnect";

export const ConsentScreen = (props: { code: string }) => {
	const [workspaceId] = useLastWorkspaceId();
	const workspacesQuery = useWorkspacesQuery();
	const hasMultipleWorkspaces = createMemo(() => (workspacesQuery.data?.workspaces.length ?? 0) > 1);
	// Once the user approves (either mode), the CLI's next verify poll one-shot
	// deletes the redis entry. Stop the query from refetching past that point —
	// otherwise a refetch races with the delete and flips the page back to
	// LinkUnavailable.
	const [approved, setApproved] = createSignal(false);
	// Landing state — the operator must consciously pick new-vs-reconnect;
	// neither form is pre-rendered because reconnect is destructive.
	const [mode, setMode] = createSignal<Mode>("choice");

	const linkQuery = createQuery<GetRunnerLinkResponse>(() => ({
		queryKey: ["runner-link", workspaceId(), props.code],
		enabled: !!workspaceId() && !approved(),
		queryFn: async () => {
			const response = await httpRequest<GetRunnerLinkResponse>(
				`${import.meta.env.VITE_BASE_URL}/api/workspace/${workspaceId()}/runner/link/${props.code}`,
				{ method: "GET" }
			);
			if (!response.ok) {
				throw new Error(response.data.error ?? "Link not found");
			}
			return response.data;
		},
		retry: false,
	}));

	return (
		<ErrorBoundary fallback={() => <LinkUnavailable showWorkspaceHint={hasMultipleWorkspaces()} />}>
			<Show when={!approved()} fallback={<ApprovedState />}>
				<Switch>
					<Match when={linkQuery.isLoading}>
						<div class="flex items-center justify-center gap-2 py-16 w-full text-grey">
							<LoadingSpinner size={20} />
							<span class="text-sm">Loading runner details...</span>
						</div>
					</Match>
					<Match when={linkQuery.isError}>
						<LinkUnavailable showWorkspaceHint={hasMultipleWorkspaces()} />
					</Match>
					<Match when={linkQuery.data}>
						<div class="mx-auto flex flex-col gap-6 w-full" style={{ "max-width": "40rem" }}>
							<MachineDetails link={linkQuery.data!} />

							<div class="h-px bg-border-color" />

							<Switch>
								<Match when={mode() === "choice"}>
									<ModeChoice onPick={setMode} />
								</Match>
								<Match when={mode() === "new"}>
									<BackBar onBack={() => setMode("choice")} label="Set up as a new runner" />
									<NewRunnerForm code={props.code} onApproved={() => setApproved(true)} />
								</Match>
								<Match when={mode() === "reconnect"}>
									<BackBar onBack={() => setMode("choice")} label="Reconnect an existing runner" />
									<ReconnectForm code={props.code} onApproved={() => setApproved(true)} />
								</Match>
							</Switch>
						</div>
					</Match>
				</Switch>
			</Show>
		</ErrorBoundary>
	);
};

const BackBar = (props: { onBack: () => void; label: string }) => (
	<div class="flex items-center gap-2">
		<button
			type="button"
			onClick={() => props.onBack()}
			class="text-grey hover:text-white text-sm flex items-center gap-1 transition-colors cursor-pointer"
		>
			<span aria-hidden>&larr;</span>
			<span>Back</span>
		</button>
		<span class="text-grey/40">/</span>
		<span class="text-white text-sm font-medium">{props.label}</span>
	</div>
);
