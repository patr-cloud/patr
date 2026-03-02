import { useParams } from "@solidjs/router";
import { createSignal, Show } from "solid-js";
import { FiTrash2 } from "solid-icons/fi";
import { useAuthState, useLastWorkspaceId } from "~/hooks/state-hooks";
import { httpRequest } from "~/utils/http-request";
import { useToast, Table, Link, CopyButton } from "~/components";
import { formatRelativeTime, get } from "~/utils/func";
import { ListContainerRepositoryManifestsResponse, ContainerRepositoryManifestInfo } from "~/bindings";
import { MaybeAccessor } from "~/utils/types";

interface ContainerImagesProps {
	manifests: MaybeAccessor<ListContainerRepositoryManifestsResponse>;
	refetch?: () => void;
}

const formatBytes = (bytes: number | bigint) => {
	if (bytes === 0) return "0 B";
	const k = 1024;
	const sizes = ["B", "KB", "MB", "GB", "TB"];
	const i = Math.floor(Math.log(Number(bytes)) / Math.log(k));
	return parseFloat((Number(bytes) / Math.pow(k, i)).toFixed(2)) + " " + sizes[i];
};

const Images = (props: ContainerImagesProps) => {
	const params = useParams();

	return (
		<div class="w-full">
			<Show
				when={get(props.manifests) && get(props.manifests).manifests && get(props.manifests).manifests.length > 0}
				fallback={
					<div class="w-full text-center py-16">
						<p class="text-white text-lg">No Images Found</p>
						<p class="text-gray-400 text-sm mt-2">
							<Link href={`/container-registry/${params.id}?tab=`} external={false} class="inline">
								Push an image
							</Link>{" "}
							to this repository to get started
						</p>
					</div>
				}
			>
				<Table
					column_grids={["flex-3", "flex-2", "flex-2", "flex-3", "flex-3", "flex-2"]}
					headings={["Tags", "Platform", "Size", "Created", "Digest", "Actions"]}
					rows={get(props.manifests).manifests}
					renderRow={(manifest) => <ImageRow manifest={manifest} refetch={props.refetch} />}
				/>
			</Show>
		</div>
	);
};

export default Images;

const ImageRow = (props: { manifest: ContainerRepositoryManifestInfo; refetch?: () => void }) => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();
	const toast = useToast();
	const params = useParams();
	const [deleteSelected, setDeleteSelected] = createSignal(false);

	const handleDelete = async (digest: string) => {
		const auth = authState();
		const wsId = workspaceId();
		const repoId = params.id;

		if (!auth || auth.type !== "LoggedIn" || !wsId || !repoId) {
			toast("User not logged in", "error");
			return;
		}

		const response = await httpRequest(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/container-registry/${repoId}/image/${digest}`,
			{
				method: "DELETE",
			}
		);

		if (!response.ok) {
			toast("Failed to delete image", "error");
			return;
		}

		toast("Image deleted successfully", "success");
		props.refetch?.();
	};

	return (
		<tr class="table-row">
			<td class="flex-3 flex items-center gap-2 overflow-hidden">
				<span class="truncate text-gray-300 font-mono text-sm" title={props.manifest.tags.join(", ")}>
					{props.manifest.tags.length > 0 ? (
						props.manifest.tags.join(", ")
					) : (
						<span class="text-gray-500 italic">No tags</span>
					)}
				</span>
			</td>
			<td class="flex-2 text-gray-400 text-sm">{props.manifest.platform}</td>
			<td class="flex-2 text-gray-400 text-sm">{formatBytes(props.manifest.size)}</td>
			<td class="flex-3 text-gray-400 text-sm">{formatRelativeTime(props.manifest.created)}</td>
			<td class="flex-3 flex items-center gap-2 overflow-hidden">
				<span class="truncate text-gray-300 font-mono text-sm max-w-[150px]">{props.manifest.digest}</span>
				<CopyButton text={props.manifest.digest} />
			</td>
			<td class="flex-2 flex items-center justify-center">
				{deleteSelected() ? (
					<div class="flex flex-row items-center gap-2">
						<button
							onClick={() => handleDelete(props.manifest.digest)}
							class="text-red-500 transition-colors mr-2 text-xs font-bold"
							title="Confirm delete"
						>
							CONFIRM
						</button>
						<button
							onClick={() => setDeleteSelected(false)}
							class="text-gray-400 transition-colors text-xs"
							title="Cancel delete"
						>
							CANCEL
						</button>
					</div>
				) : (
					<button
						onClick={() => {
							setDeleteSelected(true);
						}}
						class="text-red-500 transition-colors hover:text-red-400"
						title="Delete image"
					>
						<FiTrash2 size={16} />
					</button>
				)}
			</td>
		</tr>
	);
};
