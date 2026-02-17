import { useParams } from "@solidjs/router";
import { createSignal, Show } from "solid-js";
import { FiTrash2 } from "solid-icons/fi";
import { useAuthState, useLastWorkspaceId } from "~/hooks/state-hooks";
import { httpRequest } from "~/utils/http-request";
import { useToast, Table, Link, CopyButton } from "~/components";
import { formatRelativeTime, get } from "~/utils/func";
import { ListContainerRepositoryTagsResponse } from "~/bindings";
import { MaybeAccessor } from "~/utils/types";
import { ContainerRepositoryTagAndDigestInfo } from "~/bindings/ContainerRepositoryTagAndDigestInfo";

interface ContainerImagesProps {
	imageTags: MaybeAccessor<ListContainerRepositoryTagsResponse>;
	refetch?: () => void;
}

const Images = (props: ContainerImagesProps) => {
	const params = useParams();

	return (
		<div class="w-full">
			<Show
				when={props.imageTags && get(props.imageTags).tags && get(props.imageTags).tags.length > 0}
				fallback={
					<div class="w-full text-center py-16">
						<p class="text-white text-lg">No Images Found</p>
						<p class="text-gray-400 text-sm mt-2">
							<Link href={`/container-repositories/${params.id}?tab=`} external={false} class="inline">
								Push an image
							</Link>{" "}
							to this repository to get started
						</p>
					</div>
				}
			>
				<Table
					column_grids={["flex-4", "flex-4", "flex-4", "flex-4"]}
					headings={["Tag", "Digest", "Last Pushed", "Actions"]}
					rows={get(props.imageTags).tags}
					renderRow={(image) => <ImageRow image={image} refetch={props.refetch} />}
				/>
			</Show>
		</div>
	);
};

export default Images;

const ImageRow = (props: { image: ContainerRepositoryTagAndDigestInfo; refetch?: () => void }) => {
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
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/docker-registry/${repoId}/image/${digest}`,
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
			<td class="flex-4 flex items-center gap-2">
				<span class="truncate text-gray-300 font-mono text-sm">{props.image.tag}</span>
			</td>
			<td class="flex-4 flex items-center gap-2">
				<span class="truncate text-gray-300 font-mono text-sm">{props.image.digest}</span>
				<CopyButton text={props.image.digest} />
			</td>
			<td class="flex-4 text-gray-400 text-sm">{formatRelativeTime(props.image.lastUpdated)}</td>
			<td class="flex-4 flex items-center justify-center">
				{deleteSelected() ? (
					<div class="flex flex-row items-center gap-2">
						<button
							onClick={() => handleDelete(props.image.digest)}
							class="text-red-500 transition-colors mr-2"
							title="Confirm delete"
						>
							DELETE
						</button>
						<button
							onClick={() => setDeleteSelected(false)}
							class="text-gray-400 transition-colors"
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
						class="text-red-500 transition-colors"
						title="Delete image"
					>
						<FiTrash2 size={16} />
					</button>
				)}
			</td>
		</tr>
	);
};
