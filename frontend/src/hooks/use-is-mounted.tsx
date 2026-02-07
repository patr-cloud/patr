import { createSignal, onMount } from "solid-js";

/**
 * Hook to check if component is mounted on the client.
 * Useful for avoiding SSR hydration mismatches when rendering client-only content.
 *
 * @returns A signal that is false during SSR and initial hydration, true after mount
 */
export const useIsMounted = () => {
	const [isMounted, setIsMounted] = createSignal(false);

	onMount(() => {
		setIsMounted(true);
	});

	return isMounted;
};
