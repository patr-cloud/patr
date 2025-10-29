import { MaybeAccessor } from "~/utils/types";
import { useAuthState, useLastWorkspaceId } from "./state-hooks";
import { createMemo, createResource } from "solid-js";

interface UseFetchProps {
  id: MaybeAccessor<string>;
}

/**
 * @deprecated WIP
 */
const useFetch = (props: UseFetchProps) => {
  const [authState] = useAuthState();
  const [workspaceId] = useLastWorkspaceId();

  const fetchParams = createMemo(() => {
    return [authState(), workspaceId()] as const;
  });

  return { fetchParams };
};

export default useFetch;
