import { createEffect, ErrorBoundary, For, onCleanup, Show, Suspense } from "solid-js";
import { Button, ButtonVariant, Link, LoadingSpinner, StatusDot } from "~/components";
import { useSecretDeploymentsQuery } from "~/hooks/fetch";
import { Color } from "~/utils/color";

interface UsedByProps {
	/** The ID of the secret to list the deployments of */
	secretId: string;
	/** The saved name of the secret. Variables named anything else are highlighted. */
	secretName: string;
}

const Heading = (props: { count?: number }) => (
	<div class="flex items-center justify-between gap-md">
		<h2 class="text-white font-medium">Used by</h2>
		<Show when={props.count !== undefined}>
			<span class="text-grey text-xs">
				{props.count} {props.count === 1 ? "deployment" : "deployments"}
			</span>
		</Show>
	</div>
);

const LoadError = (props: { onRetry: () => void }) => (
	<>
		<p class="text-error text-sm mt-md">Couldn't load the deployments using this secret.</p>
		<Button variant={ButtonVariant.Plain} onClick={props.onRetry} class="text-sm mt-xs self-start cursor-pointer">
			Retry
		</Button>
	</>
);

/**
 * Everything that reads the query. Reading it suspends, so this sits inside the
 * panel's own boundaries, or a slow list would blank the secret's form too.
 *
 * A failed query doesn't throw here: `data` is just undefined. So check `isError`
 * before treating no rows as an unused secret.
 */
const UsedByList = (props: UsedByProps) => {
	const query = useSecretDeploymentsQuery(() => props.secretId);
	const deployments = () => query.data?.pages.flatMap((page) => page.deployments) ?? [];
	const totalCount = () => query.data?.pages.at(-1)?.totalCount;

	let listRef: HTMLUListElement | undefined;

	const loadMore = () => {
		if (query.hasNextPage && !query.isFetchingNextPage) query.fetchNextPage();
	};

	// A list that doesn't overflow never scrolls, so keep loading until it does.
	// Not after a failed page though, or this retries forever. That's the Retry row's job.
	const fillPanel = () => {
		if (query.isFetchNextPageError) return;
		if (listRef && listRef.scrollHeight <= listRef.clientHeight) loadMore();
	};

	createEffect(() => {
		if (query.data) fillPanel();
	});

	return (
		<>
			<Heading count={totalCount()} />
			<Show
				when={deployments().length > 0}
				fallback={
					<Show when={!query.isError} fallback={<LoadError onRetry={() => query.refetch()} />}>
						<p class="text-grey text-sm mt-md">No deployments you can view use this secret.</p>
					</Show>
				}
			>
				<div class="flex justify-between gap-md mt-md pl-3.5 pb-xs border-b border-border-color text-grey text-xs uppercase tracking-wide">
					<span>Deployment</span>
					<span>Variable</span>
				</div>
				<ul
					ref={(el) => {
						listRef = el;
						// The panel growing can leave nothing to scroll, with more still to load.
						const observer = new ResizeObserver(fillPanel);
						observer.observe(el);
						onCleanup(() => observer.disconnect());
					}}
					class="flex-1 pt-4 flex flex-col gap-2 min-h-0 overflow-y-auto"
					onScroll={(e) => {
						const el = e.currentTarget;
						if (el.scrollHeight - el.scrollTop - el.clientHeight < 40) loadMore();
					}}
				>
					<For each={deployments()}>
						{(deployment) => (
							<li class="flex items-baseline justify-between gap-md">
								<div class="flex-1 flex items-center gap-xs min-w-0">
									<span
										role="img"
										aria-label={deployment.status}
										title={deployment.status}
										class="flex"
									>
										<StatusDot status={deployment.status} />
									</span>
									<Link
										href={`/deployments/${deployment.id}`}
										color={Color.White}
										class="min-w-0 hover:underline"
									>
										<span class="truncate">{deployment.name}</span>
									</Link>
								</div>
								<div class="flex flex-col items-end gap-xxs min-w-0 max-w-1/2">
									<For each={deployment.variables}>
										{(variable) => (
											<span
												title={variable}
												class={`font-mono text-xs truncate max-w-full ${
													variable === props.secretName ? "text-grey" : "text-primary"
												}`}
											>
												{variable}
											</span>
										)}
									</For>
								</div>
							</li>
						)}
					</For>
					<Show when={query.isFetchingNextPage}>
						<li aria-hidden="true" class="flex justify-center text-grey">
							<LoadingSpinner size={16} />
						</li>
					</Show>
					<Show when={query.isFetchNextPageError && !query.isFetchingNextPage}>
						<li class="flex flex-col pb-sm">
							<LoadError onRetry={() => query.fetchNextPage()} />
						</li>
					</Show>
				</ul>
			</Show>
		</>
	);
};

/** Side panel listing the deployments using a secret, and what variable each uses it as. */
const UsedBy = (props: UsedByProps) => (
	<aside class="flex-1 w-full min-w-0 min-h-0 flex flex-col pt-md lg:pt-0 lg:px-4 border-t lg:border-t-0 lg:border-l border-border-color">
		<ErrorBoundary
			fallback={(_err, reset) => (
				<>
					<Heading />
					<LoadError onRetry={reset} />
				</>
			)}
		>
			<Suspense
				fallback={
					<>
						<Heading />
						<div class="flex justify-center py-16 text-grey">
							<LoadingSpinner size={20} />
						</div>
					</>
				}
			>
				<UsedByList secretId={props.secretId} secretName={props.secretName} />
			</Suspense>
		</ErrorBoundary>
	</aside>
);

export default UsedBy;
