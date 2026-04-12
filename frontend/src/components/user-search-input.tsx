import { createSignal, For, Show, onMount, onCleanup, untrack } from "solid-js";
import { WithId } from "~/bindings/WithId";
import { BasicUserInfo } from "~/bindings/BasicUserInfo";
import { useUserSearchQuery } from "~/hooks/fetch";

interface UserSearchInputProps {
	placeholder?: string;
	onUserSelect: (user: WithId<BasicUserInfo>) => void;
	class?: string;
	value?: string;
	onClear?: () => void;
}

export const UserSearchInput = (props: UserSearchInputProps) => {
	const [searchQuery, setSearchQuery] = createSignal(untrack(() => props.value || ""));
	const [showDropdown, setShowDropdown] = createSignal(false);
	const [selectedUser, setSelectedUser] = createSignal<WithId<BasicUserInfo> | null>(null);
	let inputRef: HTMLInputElement | undefined;
	let dropdownRef: HTMLDivElement | undefined;

	// Don't search when user is already selected
	const effectiveQuery = () => (selectedUser() ? "" : searchQuery());
	const searchResults = useUserSearchQuery(effectiveQuery);

	const handleUserSelect = (user: WithId<BasicUserInfo>) => {
		setSelectedUser(user);
		setSearchQuery(`${user.firstName} ${user.lastName} (@${user.username})`);
		setShowDropdown(false);
		props.onUserSelect(user);
	};

	const handleInputChange = (e: Event) => {
		const value = (e.currentTarget as HTMLInputElement).value;
		setSearchQuery(value);
		setSelectedUser(null);
		setShowDropdown(value.length >= 2);
	};

	const handleClickOutside = (e: Event) => {
		const mouseEvent = e as MouseEvent;
		if (
			inputRef &&
			dropdownRef &&
			!inputRef.contains(mouseEvent.target as Node) &&
			!dropdownRef.contains(mouseEvent.target as Node)
		) {
			setShowDropdown(false);
		}
	};

	onMount(() => {
		document.addEventListener("click", handleClickOutside);
	});

	onCleanup(() => {
		document.removeEventListener("click", handleClickOutside);
	});

	return (
		<div class={`relative ${props.class || ""}`}>
			<input
				ref={inputRef}
				type="text"
				placeholder={props.placeholder || "Search for user..."}
				value={searchQuery()}
				onInput={handleInputChange}
				onFocus={() => searchQuery().length >= 2 && setShowDropdown(true)}
				class="w-full px-4 py-2 bg-secondary-light border border-border-color rounded text-white placeholder-gray-400 focus:outline-none focus:border-primary"
			/>

			<Show when={showDropdown() && searchQuery().length >= 2}>
				<div
					ref={dropdownRef}
					class="absolute z-50 w-full mt-1 bg-secondary-light border border-border-color rounded shadow-lg max-h-60 overflow-y-auto"
				>
					<Show
						when={searchResults.data?.users && searchResults.data!.users.length > 0}
						fallback={<div class="px-4 py-3 text-gray-400 text-sm">No users found</div>}
					>
						<For each={searchResults.data!.users}>
							{(user) => (
								<button
									type="button"
									onClick={() => handleUserSelect(user)}
									class="w-full px-4 py-3 text-left hover:bg-secondary transition-colors border-b border-border-color last:border-b-0"
								>
									<div class="flex flex-col gap-1">
										<div class="text-white font-medium">
											{user.firstName} {user.lastName}
										</div>
										<div class="text-gray-400 text-sm">@{user.username}</div>
									</div>
								</button>
							)}
						</For>
					</Show>
				</div>
			</Show>
		</div>
	);
};
