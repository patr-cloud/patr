import { createEffect, ErrorBoundary, For, Show, Suspense } from "solid-js";
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

/**
 * Everything that reads the query. Reading it suspends and throws, so this sits
 * inside the panel's own boundaries, or a slow or failed list would take the
 * secret's form down with it.
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
	createEffect(() => {
		if (query.data && listRef && listRef.scrollHeight <= listRef.clientHeight) loadMore();
	});

	return (
		<>
			<Heading count={totalCount()} />
			<Show
				when={deployments().length > 0}
				fallback={<p class="text-grey text-sm mt-md">No deployments you can view use this secret.</p>}
			>
				<div class="flex justify-between gap-md mt-md pl-3.5 pb-xs border-b border-border-color text-grey text-xs uppercase tracking-wide">
					<span>Deployment</span>
					<span>Variable</span>
				</div>
				<ul
					ref={listRef}
					class="flex-1 min-h-0 overflow-y-auto"
					onScroll={(e) => {
						const el = e.currentTarget;
						if (el.scrollHeight - el.scrollTop - el.clientHeight < 40) loadMore();
					}}
				>
					<For each={deployments()}>
						{(deployment) => (
							<li class="flex items-baseline justify-between gap-md py-sm">
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
						<li class="flex justify-center py-sm text-grey">
							<LoadingSpinner size={16} />
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
					<p class="text-error text-sm mt-md">Couldn't load the deployments using this secret.</p>
					<Button
						variant={ButtonVariant.Plain}
						onClick={reset}
						class="text-sm mt-xs self-start cursor-pointer"
					>
						Retry
					</Button>
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
