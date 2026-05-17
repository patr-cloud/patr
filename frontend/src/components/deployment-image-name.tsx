import { Show } from "solid-js";
import { Deployment, WithId } from "~/bindings";
import { Tooltip } from "~/components";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { useContainerRegistryInfoQuery } from "~/hooks/fetch";
import { REGISTRY_DOMAIN } from "~/utils/env";

const DeploymentImageName = (props: { item: WithId<Deployment> }) => {
	const [workspaceId] = useLastWorkspaceId();
	const isExternal = () => "imageName" in props.item;
	const repositoryId = () => (props.item as { repositoryId?: string }).repositoryId;

	const repoInfoQuery = useContainerRegistryInfoQuery(() => repositoryId() ?? "");

	const fullImage = () => {
		if (isExternal()) {
			return `${props.item.registry}/${(props.item as { imageName: string }).imageName}:${props.item.imageTag}`;
		}
		const repoName = repoInfoQuery.data?.repository.name ?? "...";
		const registryPrefix = REGISTRY_DOMAIN ? `${REGISTRY_DOMAIN}/` : "";
		return `${registryPrefix}${workspaceId()}/${repoName}:${props.item.imageTag}`;
	};

	return (
		<Tooltip content={fullImage()} class="min-w-0">
			<span class="truncate font-log text-xs text-grey block">
				<Show
					when={isExternal() || !repoInfoQuery.isFetching}
					fallback={<span class="animate-pulse">{fullImage()}</span>}
				>
					{fullImage()}
				</Show>
			</span>
		</Tooltip>
	);
};

export default DeploymentImageName;
