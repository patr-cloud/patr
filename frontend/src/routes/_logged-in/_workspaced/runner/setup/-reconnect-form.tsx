import { useMutation } from "@tanstack/solid-query";
import { createSignal, For, Match, Switch } from "solid-js";
import { ReconnectRunnerLinkResponse } from "~/bindings";
import { Alert, Button, ButtonVariant, LoadingSpinner, useToast } from "~/components";
import StatusChip from "~/components/status-chip";
import { useRunnersListQuery } from "~/hooks/fetch";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { httpRequest } from "~/utils/http-request";
import { formatRelativeTime } from "~/utils/func";

/**
 * "Reconnect" mode: pick one of the workspace's existing runners and rotate its
 * credentials onto this machine. Only runners that aren't currently connected
 * can be picked — reconnecting a live one would rotate the token out from under
 * it. The rotation is destructive, hence the warning.
 */
export const ReconnectForm = (props: { code: string; onApproved: () => void }) => {
	const [workspaceId] = useLastWorkspaceId();
	const [selected, setSelected] = createSignal<string | null>(null);
	const toast = useToast();

	// A workspace realistically has a small number of runners; pull a generous
	// page so the picker shows them all without its own pagination.
	const runnersQuery = useRunnersListQuery(
		() => undefined,
		() => "100"
	);

	const reconnectMutation = useMutation(() => ({
		mutationFn: async (runnerId: string) => {
			const wsId = workspaceId();
			const response = await httpRequest<ReconnectRunnerLinkResponse>(
				`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/runner/link/${props.code}/reconnect/${runnerId}`,
				{ method: "POST" }
			);
			if (!response.ok) {
				throw new Error(response.data.error ?? "Reconnect failed");
			}
			return response.data;
		},
		onSuccess: () => props.onApproved(),
		onError: (err: Error) => toast(err.message, "error"),
	}));

	const onReconnect = () => {
		const id = selected();
		if (!id) return;
		if (!workspaceId()) {
			toast("No workspace selected. Pick one in the sidebar.", "error");
			return;
		}
		reconnectMutation.mutate(id);
	};

	return (
		<div class="flex flex-col gap-6">
			<Switch>
				<Match when={runnersQuery.isLoading}>
					<div class="flex items-center justify-center gap-2 py-12 text-grey">
						<LoadingSpinner size={20} />
						<span class="text-sm">Loading runners...</span>
					</div>
				</Match>
				<Match when={(runnersQuery.data?.runners.length ?? 0) === 0}>
					<p class="text-grey text-sm text-center py-8">
						No runners in this workspace yet — go back and choose&nbsp;
						<span class="text-white font-medium">New&nbsp;runner</span>.
					</p>
				</Match>
				<Match when={runnersQuery.data}>
					<div role="radiogroup" class="flex flex-col gap-2">
						<For each={runnersQuery.data!.runners}>
							{(runner) => {
								const disabled = () => runner.connected;
								const isSelected = () => selected() === runner.id;
								return (
									<button
										type="button"
										role="radio"
										aria-checked={isSelected()}
										disabled={disabled()}
										onClick={() => !disabled() && setSelected(runner.id)}
										class={`flex items-center gap-4 text-left px-4 py-3 rounded-xs border transition-colors ${
											isSelected()
												? "border-primary bg-primary/5"
												: "border-border-color hover:border-grey/40"
										} ${disabled() ? "opacity-50 cursor-not-allowed hover:border-border-color" : "cursor-pointer"}`}
									>
										<div class="flex flex-col gap-1 min-w-0 flex-1">
											<span class="text-white text-sm font-medium truncate">{runner.name}</span>
											<span class="text-grey/60 text-xxs font-log truncate">{runner.id}</span>
										</div>
										<div class="flex flex-col items-end gap-1 shrink-0">
											<StatusChip status={runner.connected ? "connected" : "unreachable"} />
											<span class="text-grey/60 text-xxs">
												{runner.connected
													? "disconnect it first"
													: runner.lastSeen
														? `last seen ${formatRelativeTime(runner.lastSeen)}`
														: "never connected"}
											</span>
										</div>
									</button>
								);
							}}
						</For>
					</div>

					<Alert
						type="warning"
						align="center"
						message="Reconnecting rotates this runner's credentials. Any process still using the old ones will stop working the next time it contacts Patr — only this machine will connect as the runner from now on."
					/>

					<div class="flex justify-end">
						<Button
							variant={ButtonVariant.Contained}
							disabled={!selected()}
							loading={reconnectMutation.isPending}
							loadingContent={() => <span>Reconnecting...</span>}
							onClick={onReconnect}
						>
							Reconnect
						</Button>
					</div>
				</Match>
			</Switch>
		</div>
	);
};
