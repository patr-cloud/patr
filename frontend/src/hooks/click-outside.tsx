import { createEffect, onCleanup } from "solid-js";
import { get } from "~/utils/func";
import { MaybeAccessor } from "~/utils/types";

export interface Callback<T extends Event = Event> {
	(event: T): void;
}

const useClickOutside = (ref: MaybeAccessor<HTMLElement | undefined>, handler: Callback) => {
	createEffect(() => {
		const listener = (event: MouseEvent) => {
			const ele = get(ref);
			// Do nothing if clicking ref's element or descendent elements
			if (!ele || ele.contains(event.target as Node)) {
				return;
			}
			handler(event);
		};

		document.addEventListener("mousedown", listener);

		onCleanup(() => {
			document.removeEventListener("mousedown", listener);
		});
	});
};

export default useClickOutside;
