import { MaybeAccessor } from "~/utils/types";
import { useAuthState, useLastWorkspaceId } from "../state-hooks";
import { createMemo } from "solid-js";

interface UseFetchProps {
  id: MaybeAccessor<string>;
}

/**
 * @deprecated This hook is incomplete and will be removed in a future release.  
 * Please use your own data fetching logic or an alternative hook if available.  
 * Removal planned for during cleanup

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
