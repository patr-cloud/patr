import { createFileRoute } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import { ErrorBoundary, Show } from "solid-js";
import {
	Button,
	ButtonVariant,
	LoadingSpinner,
	PageContainer,
	PageContainerBody,
	PageContainerHead,
	StatusBadge,
	ToggleSwitch,
	UsageBar,
} from "~/components";
import { Color } from "~/utils/color";
import { formatSize } from "~/utils/func";
import { useContainerRegistryUsageQuery } from "~/hooks/fetch";

const ContainerRegistrySettings = () => {
	const usageQuery = useContainerRegistryUsageQuery();

	return (
		<>
			<Title>Registry Settings | Patr</Title>
			<PageContainer>
				<PageContainerHead
					breadcrumbs={[
						{ label: "Container Repositories", url: "/container-registry" },
						{ label: "Settings" },
					]}
					subText="Storage and cleanup settings for your whole registry"
				/>

				<PageContainerBody class="flex flex-col gap-8">
					<ErrorBoundary
						fallback={(err, reset) => (
							<div class="flex flex-col items-center justify-center gap-4 py-16">
								<p class="text-error text-sm">Error loading settings: {err.message}</p>
								<Button variant={ButtonVariant.Outlined} onClick={reset}>
									Retry
								</Button>
							</div>
						)}
					>
						<section class="flex flex-col gap-4 max-w-200">
							<div>
								<h2 class="text-white text-base font-medium">Storage used</h2>
								<p class="text-gray-400 text-sm mt-1">
									Total space taken up by every image across all your repositories. Layers shared
									between images are counted once.
								</p>
							</div>

							<Show
								when={usageQuery.data}
								fallback={
									<div class="flex items-center gap-2 py-4 text-grey">
										<LoadingSpinner size={16} />
										<span class="text-sm">Loading usage…</span>
									</div>
								}
							>
								{(usage) => (
									<div class="flex flex-col gap-3">
										<div class="flex items-end justify-between">
											<span class="text-white text-2xl font-semibold">
												{formatSize(usage().usedBytes)}
											</span>
											<span class="text-gray-400 text-sm">
												{Number(usage().repositoryCount)}{" "}
												{Number(usage().repositoryCount) === 1 ? "repository" : "repositories"} ·{" "}
												{Number(usage().imageCount)}{" "}
												{Number(usage().imageCount) === 1 ? "image" : "images"}
											</span>
										</div>
										<UsageBar value={Number(usage().usedBytes)} />
									</div>
								)}
							</Show>
						</section>

						<section class="flex flex-col gap-4 max-w-200 border-t border-border-color pt-8 opacity-60">
							<div class="flex items-center gap-3">
								<h2 class="text-white text-base font-medium">Auto-clean untagged images</h2>
								<StatusBadge text="Coming soon" color={Color.Warning} />
							</div>
							<p class="text-gray-400 text-sm">
								Automatically delete images that no version label points at, once they're older
								than a set number of days — keeps your storage tidy without manual cleanup.
							</p>

							<div class="flex items-center gap-4">
								<ToggleSwitch checked={false} disabled label="Enable auto-clean" />
							</div>

							<div class="flex items-center gap-3">
								<span class="text-gray-400 text-sm">Delete untagged images older than</span>
								<span class="w-16 px-3 py-2 rounded-xs bg-secondary-medium border border-border-color text-gray-400 text-sm text-center select-none">
									30
								</span>
								<span class="text-gray-400 text-sm">days</span>
							</div>
						</section>
					</ErrorBoundary>
				</PageContainerBody>
			</PageContainer>
		</>
	);
};

export const Route = createFileRoute("/_logged-in/_workspaced/container-registry/settings")({
	component: ContainerRegistrySettings,
});
