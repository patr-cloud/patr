import { useNavigate } from "@tanstack/solid-router";
import { createSignal, For, Show } from "solid-js";
import { FiChevronRight, FiTrash2 } from "solid-icons/fi";
import {
	CopyableField,
	CopyableFieldVariant,
	ExpandableRow,
	Link,
	LoadingSpinner,
	Table,
	useToast,
} from "~/components";
import { formatRelativeTime, formatSize, get } from "~/utils/func";
import { httpRequest } from "~/utils/http-request";
import { useAuthState, useLastWorkspaceId } from "~/hooks/state-hooks";
import { useContainerManifestDetailsQuery } from "~/hooks/fetch";
import { ContainerRepositoryManifestInfo, ListContainerRepositoryManifestsResponse } from "~/bindings";
import { MaybeAccessor } from "~/utils/types";
import { formatPlatform, KindBadge, PlatformSummary, shortDigest } from "./registry-ui";

const HEADINGS = ["Tags", "Platform", "Size", "Created", ""];
const COLUMN_GRIDS = ["flex-3", "flex-3", "flex-2", "flex-2", "flex-2"];

interface VersionsProps {
	repoId: string;
	/** The base image path, e.g. `registry.patr.cloud/<workspace>/<repo>`. */
	imagePath: string;
	manifests: MaybeAccessor<ListContainerRepositoryManifestsResponse>;
	refetch?: () => void;
}

/**
 * The "Images" tab: a tag-centric list of everything pushed to a repository.
 * Single-platform images and artifacts are plain rows that open a detail page;
 * a multi-arch index expands in place to its per-platform children. Manifests
 * with no tag pointing at them are tucked into a collapsed "Untagged" section.
 */
const Versions = (props: VersionsProps) => {
	const all = () => get(props.manifests).manifests ?? [];
	const tagged = () => all().filter((manifest) => manifest.tags.length > 0);
	const untagged = () => all().filter((manifest) => manifest.tags.length === 0);
	const [showUntagged, setShowUntagged] = createSignal(false);

	return (
		<div class="w-full flex flex-col gap-6">
			<Show
				when={all().length > 0}
				fallback={
					<div class="w-full text-center py-16">
						<p class="text-white text-lg">No images yet</p>
						<p class="text-gray-400 text-sm mt-2">
							<Link href={`/container-registry/${props.repoId}?tab=`} external={false} class="inline">
								Push an image
							</Link>{" "}
							to this repository to get started.
						</p>
					</div>
				}
			>
				<Table
					column_grids={COLUMN_GRIDS}
					headings={HEADINGS}
					rows={tagged()}
					renderRow={(manifest) => (
						<VersionRow
							manifest={manifest}
							repoId={props.repoId}
							imagePath={props.imagePath}
							refetch={props.refetch}
						/>
					)}
				/>

				<Show when={untagged().length > 0}>
					<div class="flex flex-col gap-2">
						<button
							type="button"
							onClick={() => setShowUntagged((value) => !value)}
							class="flex items-center gap-2 text-sm text-gray-300 hover:text-white w-fit"
						>
							<FiChevronRight
								size={16}
								class={`transition-transform ${showUntagged() ? "rotate-90" : ""}`}
							/>
							Untagged ({untagged().length})
						</button>
						<p class="text-xs text-gray-500 pl-6">
							Images with no version label pointing at them — usually older builds a tag has since moved
							away from.
						</p>
						<Show when={showUntagged()}>
							<Table
								column_grids={COLUMN_GRIDS}
								headings={HEADINGS}
								rows={untagged()}
								renderRow={(manifest) => (
									<VersionRow
										manifest={manifest}
										repoId={props.repoId}
										imagePath={props.imagePath}
										refetch={props.refetch}
									/>
								)}
							/>
						</Show>
					</div>
				</Show>
			</Show>
		</div>
	);
};

export default Versions;

/** A single top-level row: an expandable index, or a plain image/artifact row. */
const VersionRow = (props: {
	manifest: ContainerRepositoryManifestInfo;
	repoId: string;
	imagePath: string;
	refetch?: () => void;
}) => {
	const navigate = useNavigate();
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();
	const toast = useToast();

	// Prefer a tag in the URL when the image has one; fall back to the digest for
	// untagged manifests.
	const goToDetail = () =>
		navigate({
			to: "/container-registry/$id/manifest/$digest",
			params: { id: props.repoId, digest: props.manifest.tags[0] ?? props.manifest.digest },
		});

	const handleDelete = async () => {
		const auth = authState();
		const wsId = workspaceId();
		if (!auth || auth.type !== "LoggedIn" || !wsId) {
			toast("User not logged in", "error");
			return;
		}
		const response = await httpRequest(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/container-registry/${props.repoId}/manifest/${props.manifest.digest}`,
			{ method: "DELETE" }
		);
		if (!response.ok) {
			toast("Failed to delete image", "error");
			return;
		}
		toast("Image deleted successfully", "success");
		props.refetch?.();
	};

	return (
		<Show
			when={props.manifest.kind === "index"}
			fallback={
				<tr
					role="row"
					tabIndex={0}
					class="table-row cursor-pointer focus-visible:outline-primary"
					onClick={goToDetail}
					onKeyDown={(e) => {
						if (e.key === "Enter" || e.key === " ") {
							e.preventDefault();
							goToDetail();
						}
					}}
				>
					<RowCells manifest={props.manifest} onDelete={handleDelete} />
				</tr>
			}
		>
			<ExpandableRow
				summary={(open) => (
					<RowCells manifest={props.manifest} open={open} showChevron onDelete={handleDelete} />
				)}
			>
				<IndexPanel repoId={props.repoId} manifest={props.manifest} />
			</ExpandableRow>
		</Show>
	);
};

/** The cells shared by every version row, with an optional leading chevron. */
const RowCells = (props: {
	manifest: ContainerRepositoryManifestInfo;
	showChevron?: boolean;
	open?: boolean;
	onDelete: () => void;
}) => {
	return (
		<>
			<td role="cell" class="flex-3 min-w-0">
				<div class="flex items-center gap-2 w-full min-w-0">
					<span class="w-4 shrink-0 flex items-center justify-center">
						<Show when={props.showChevron}>
							<FiChevronRight
								size={16}
								class={`text-gray-400 transition-transform ${props.open ? "rotate-90" : ""}`}
							/>
						</Show>
					</span>
					<Show
						when={props.manifest.tags.length > 0}
						fallback={
							<span class="flex items-center gap-1 min-w-0 text-gray-500">
								<span class="italic shrink-0">Untagged ·</span>
								<CopyableField
									variant={CopyableFieldVariant.Text}
									value={props.manifest.digest}
									innerClass="truncate max-w-40"
								/>
							</span>
						}
					>
						<span class="flex flex-wrap gap-1 min-w-0">
							<For each={props.manifest.tags}>
								{(tag) => (
									<CopyableField variant={CopyableFieldVariant.Text} value={tag} class="chip-tag" />
								)}
							</For>
						</span>
					</Show>
				</div>
			</td>
			<td role="cell" class="flex-3 min-w-0 flex items-center gap-2">
				<PlatformSummary kind={props.manifest.kind} platforms={props.manifest.platforms} />
				<KindBadge kind={props.manifest.kind} />
			</td>
			<td role="cell" class="flex-2 text-gray-400 text-sm">
				{formatSize(props.manifest.size)}
			</td>
			<td role="cell" class="flex-2 text-gray-400 text-sm">
				{formatRelativeTime(props.manifest.created)}
			</td>
			<td role="cell" class="flex-2 flex items-center justify-center">
				<DeleteButton onDelete={props.onDelete} />
			</td>
		</>
	);
};

/** The inline two-step delete confirmation used on each version row. */
const DeleteButton = (props: { onDelete: () => void }) => {
	const [confirming, setConfirming] = createSignal(false);

	return (
		<Show
			when={confirming()}
			fallback={
				<button
					type="button"
					onClick={(e) => {
						e.stopPropagation();
						setConfirming(true);
					}}
					class="text-error transition-colors hover:text-error-light"
					title="Delete image"
				>
					<FiTrash2 size={16} />
				</button>
			}
		>
			<div class="flex flex-row items-center gap-2">
				<button
					type="button"
					onClick={(e) => {
						e.stopPropagation();
						props.onDelete();
						setConfirming(false);
					}}
					class="text-error transition-colors text-xs font-bold"
					title="Confirm delete"
				>
					CONFIRM
				</button>
				<button
					type="button"
					onClick={(e) => {
						e.stopPropagation();
						setConfirming(false);
					}}
					class="text-gray-400 transition-colors text-xs"
					title="Cancel delete"
				>
					Cancel
				</button>
			</div>
		</Show>
	);
};

/** The panel revealed under a multi-arch index: its per-platform children. */
const IndexPanel = (props: { repoId: string; manifest: ContainerRepositoryManifestInfo }) => {
	const navigate = useNavigate();
	const detailsQuery = useContainerManifestDetailsQuery(
		() => props.repoId,
		() => props.manifest.digest
	);

	return (
		<div class="flex flex-col gap-3">
			<p class="text-sm text-gray-400">
				This version is multi-arch — it bundles one image per platform. Pick a platform to see its details.
			</p>
			<Show
				when={detailsQuery.isSuccess}
				fallback={
					<div class="flex items-center gap-2 text-grey text-sm">
						<LoadingSpinner size={16} />
						Loading platforms…
					</div>
				}
			>
				<div class="flex flex-col gap-2">
					<For each={detailsQuery.data?.referencedManifests ?? []}>
						{(child) => (
							<button
								type="button"
								onClick={() =>
									navigate({
										to: "/container-registry/$id/manifest/$digest",
										params: { id: props.repoId, digest: child.digest },
									})
								}
								class="flex items-center justify-between gap-4 rounded-xs bg-secondary-light hover:bg-secondary-medium px-md py-2 text-left focus-visible:outline-primary"
							>
								<span class="font-mono text-gray-300 w-36 shrink-0">
									{child.platforms[0] ? formatPlatform(child.platforms[0]) : "unknown"}
								</span>
								<span class="font-mono text-gray-500 truncate flex-1 min-w-0">
									{shortDigest(child.digest)}
								</span>
								<span class="text-gray-400 shrink-0">{formatSize(child.size)}</span>
							</button>
						)}
					</For>
				</div>
			</Show>
		</div>
	);
};
