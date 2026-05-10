import { createSignal, untrack } from "solid-js";
import { WithId } from "~/bindings/WithId";
import { BasicUserInfo } from "~/bindings/BasicUserInfo";

interface UserSearchInputProps {
	placeholder?: string;
	onUserSelect: (user: WithId<BasicUserInfo>) => void;
	class?: string;
	value?: string;
	onClear?: () => void;
}

/// User picker — paste a user ID. Search is gone; we add by ID directly.
export const UserSearchInput = (props: UserSearchInputProps) => {
	const [userId, setUserId] = createSignal(untrack(() => props.value || ""));

	const handleInputChange = (e: Event) => {
		const value = (e.currentTarget as HTMLInputElement).value.trim();
		setUserId(value);

		if (value.length === 0) {
			props.onClear?.();
			return;
		}

		// Resolve as a synthetic WithId<BasicUserInfo>; the backend lookup happens
		// when the parent submits the add-user-to-workspace request.
		props.onUserSelect({
			id: value,
			firstName: "",
			lastName: "",
		} as WithId<BasicUserInfo>);
	};

	return (
		<div class={`relative ${props.class || ""}`}>
			<input
				type="text"
				placeholder={props.placeholder || "Paste user ID"}
				value={userId()}
				onInput={handleInputChange}
				class="w-full px-4 py-2 bg-secondary-light border border-border-color rounded text-white placeholder-gray-400 focus:outline-none focus:border-primary font-mono text-sm"
			/>
			<p class="mt-1 text-gray-500 text-xs">
				Ask the user for their ID — they can find it in their profile page.
			</p>
		</div>
	);
};
