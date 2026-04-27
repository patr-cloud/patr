import { createQuery } from "@tanstack/solid-query";
import { createMemo, createSignal, ErrorBoundary, Match, Show, Switch } from "solid-js";
import { GetRunnerLinkResponse } from "~/bindings";
import { LoadingSpinner } from "~/components";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { useWorkspacesQuery } from "~/hooks/fetch";
import { httpRequest } from "~/utils/http-request";
import { ApprovalForm } from "./-approval-form";
import { ApprovedState } from "./-approved-state";
import { LinkUnavailable } from "./-link-unavailable";

export const ConsentScreen = (props: { code: string }) => {
	const [workspaceId] = useLastWorkspaceId();
	const workspacesQuery = useWorkspacesQuery();
	const hasMultipleWorkspaces = createMemo(() => (workspacesQuery.data?.workspaces.length ?? 0) > 1);
	// Once the user clicks Approve, the CLI's next verify poll one-shot deletes
	// the redis entry. Stop the query from refetching past that point — otherwise
	// a refetch races with the delete and flips the page back to LinkUnavailable.
	const [approved, setApproved] = createSignal(false);

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
						<ApprovalForm link={linkQuery.data!} code={props.code} onApproved={() => setApproved(true)} />
					</Match>
				</Switch>
			</Show>
		</ErrorBoundary>
	);
};
