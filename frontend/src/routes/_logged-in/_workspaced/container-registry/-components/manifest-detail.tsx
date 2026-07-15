import { Accessor, For, JSX, Show } from "solid-js";
import { CopyableField, CopyableFieldVariant, LoadingSpinner, Tooltip } from "~/components";
import { formatDate, formatRelativeTime, formatSize } from "~/utils/func";
import { useContainerExposedPortsQuery, useContainerManifestDetailsQuery } from "~/hooks/fetch";
import { formatPlatform, KindBadge, PullCommand } from "./registry-ui";

interface ManifestDetailProps {
	repoId: Accessor<string>;
	/** The digest (or tag) being viewed. */
	reference: Accessor<string>;
	/** The base image path, e.g. `registry.patr.cloud/<workspace>/<repo>`. */
	imagePath: Accessor<string>;
}

/**
 * The digest-detail view: how to pull this exact image, what platforms it
 * covers, the ports it listens on, and the filesystem layers it's built from —
 * the plain-language answer to "what is this thing and what's inside it".
 */
const ManifestDetail = (props: ManifestDetailProps) => {
	const detailsQuery = useContainerManifestDetailsQuery(
		() => props.repoId(),
		() => props.reference()
	);
	const portsQuery = useContainerExposedPortsQuery(
		() => props.repoId(),
		() => props.reference()
	);

	// Exposed ports are best-effort: gate on `isSuccess` so a pending or failed
	// ports request can never suspend or error the whole page (reading `.data`
	// on a not-yet-resolved query suspends under Solid Suspense).
	const ports = () => (portsQuery.isSuccess ? (portsQuery.data?.ports ?? []) : []);

	return (
		<Show
			when={detailsQuery.data}
			fallback={
				<div class="flex items-center justify-center gap-2 py-16 text-grey">
					<LoadingSpinner size={20} />
					<span class="text-sm">Loading image details…</span>
				</div>
			}
		>
			{(details) => {
				const primaryTag = () => details().tags[0];
				const byTag = () => (primaryTag() ? `${props.imagePath()}:${primaryTag()}` : undefined);
				const byDigest = () => `${props.imagePath()}@${details().digest}`;
				const layers = () => details().layers ?? [];

				return (
					<div class="flex flex-col gap-8">
						<div class="flex flex-col gap-8">
							<div class="flex items-center gap-3 flex-wrap">
								<Show
									when={details().tags.length > 0}
									fallback={<span class="text-gray-500 italic">Untagged</span>}
								>
									<span class="text-gray-500 text-sm">Tags:</span>
									<For each={details().tags}>
										{(tag) => (
											<CopyableField
												variant={CopyableFieldVariant.Text}
												value={tag}
												class="chip-tag"
											/>
										)}
									</For>
								</Show>
								<KindBadge kind={details().kind} />
							</div>

							<div class="flex items-center justify-between w-full text-sm">
								<span class="flex items-center gap-2">
									<span class="text-gray-500">Size</span>
									<span class="text-white">{formatSize(details().size)}</span>
								</span>
								<span class="flex items-center gap-2">
									<span class="text-gray-500">Created</span>
									<Tooltip content={formatDate(details().created)} class="text-white">
										<span class="text-white">{formatRelativeTime(details().created)}</span>
									</Tooltip>
								</span>
								<span class="flex items-center gap-2">
									<span class="text-gray-500">Platforms</span>
									<Show
										when={details().platforms.length > 0}
										fallback={<span class="text-gray-500">—</span>}
									>
										<span class="flex flex-wrap gap-1">
											<For each={details().platforms}>
												{(platform) => <span class="chip-tag">{formatPlatform(platform)}</span>}
											</For>
										</span>
									</Show>
								</span>
							</div>
						</div>

						<section class="flex flex-col gap-3">
							<h2 class="text-white text-base font-medium">Pull this image</h2>
							<Show when={byTag()}>
								<PullCommand reference={byTag() as string} label="By version" />
							</Show>
							<PullCommand reference={byDigest()} label="By exact ID (never changes)" />
						</section>

						<div class="flex flex-col gap-4 max-w-2xl">
							<DetailField label="ID (digest)">
								<CopyableField
									variant={CopyableFieldVariant.Input}
									value={details().digest}
									innerClass="font-mono"
								/>
							</DetailField>
							<Show when={details().artifactType}>
								<DetailField label="Artifact type">
									<span class="font-mono text-white text-sm break-all">{details().artifactType}</span>
								</DetailField>
							</Show>
						</div>

						<Show when={ports().length > 0}>
							<section class="flex flex-col gap-2">
								<h2 class="text-white text-base font-medium">Ports this image listens on</h2>
								<div class="flex flex-wrap gap-2">
									<For each={ports()}>{(port) => <span class="chip-tag">{port}</span>}</For>
								</div>
							</section>
						</Show>

						<Show when={layers().length > 0}>
							<section class="flex flex-col gap-2">
								<h2 class="text-white text-base font-medium">Layers</h2>
								<p class="text-xs text-gray-500">
									The stacked filesystem pieces that make up this image. Each is stored once and
									shared with any image that reuses it.
								</p>
								<div class="flex flex-col gap-1 mt-1">
									<For each={layers()}>
										{(layer, index) => (
											<div class="flex items-center justify-between gap-4 rounded-xs bg-secondary-light px-md py-2">
												<span class="text-gray-500 shrink-0 w-8 text-sm">{index() + 1}</span>
												<CopyableField
													variant={CopyableFieldVariant.Text}
													value={layer.digest}
													class="flex-1 min-w-0"
													innerClass="truncate text-sm"
												/>
												<span class="text-gray-400 shrink-0 text-sm">
													{formatSize(layer.size)}
												</span>
											</div>
										)}
									</For>
								</div>
							</section>
						</Show>
					</div>
				);
			}}
		</Show>
	);
};

export default ManifestDetail;

/** A labelled read-only field, matching the Overview tab's layout. */
const DetailField = (props: { label: string; children: JSX.Element }) => (
	<div class="flex flex-col gap-1">
		<span class="text-xs text-gray-500">{props.label}</span>
		{props.children}
	</div>
);
