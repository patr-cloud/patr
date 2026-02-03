import { ActionTypes, MaybeAccessor, ResourceTypes } from "~/utils/types";
import { useLastWorkspaceId } from "../state-hooks";
import { createMemo } from "solid-js";
import { get } from "~/utils/func";

const useIsAllowed = (
	resourceType: ResourceTypes,
	action: ActionTypes,
	resId?: MaybeAccessor<string>,
	getResourceIds: boolean = false
) => {
	const [workspaceId] = useLastWorkspaceId();

	const isAllowed = createMemo(() => {
		const resourceId = get(resId);
		const wsId = get(workspaceId);

		if (!wsId) return false;
	});
};

export default useIsAllowed;
