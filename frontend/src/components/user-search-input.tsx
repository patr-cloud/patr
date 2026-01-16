import { createSignal, createResource, For, Show, onMount, onCleanup, Suspense } from "solid-js";
import { SearchForUserResponse } from "~/bindings/SearchForUserResponse";
import { WithId } from "~/bindings/WithId";
import { BasicUserInfo } from "~/bindings/BasicUserInfo";
import { httpRequest } from "~/utils/http-request";

interface UserSearchInputProps {
	placeholder?: string;
	onUserSelect: (user: WithId<BasicUserInfo>) => void;
	accessToken: string;
	class?: string;
	value?: string;
	onClear?: () => void;
}

export const UserSearchInput = (props: UserSearchInputProps) => {
	const [searchQuery, setSearchQuery] = createSignal(props.value || "");
	const [showDropdown, setShowDropdown] = createSignal(false);
	const [selectedUser, setSelectedUser] = createSignal<WithId<BasicUserInfo> | null>(null);
	let inputRef: HTMLInputElement | undefined;
	let dropdownRef: HTMLDivElement | undefined;

	const [searchResults] = createResource(searchQuery, async (query) => {
		if (!query || query.length < 2) {
			return { users: [] };
		}

		// Don't search if we have a selected user
		if (selectedUser()) {
			return { users: [] };
		}

		console.log("Searching for users with query:", query);

		const response = await httpRequest<SearchForUserResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/user/search?query=${encodeURIComponent(query)}`,
			{
				method: "GET",
				headers: {
					"Content-Type": "application/json",
					Authorization: `Bearer ${props.accessToken}`,
				},
			}
		);

		if (!response.ok) {
			console.error("Failed to search users:", response.data.error);
			return { users: [] };
		}

		console.log("Search results:", response.data);
		return response.data;
	});

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
					<Suspense fallback={<div class="px-4 py-3 text-gray-400 text-sm">Searching...</div>}>
						<Show
							when={searchResults()?.users && searchResults()!.users.length > 0}
							fallback={<div class="px-4 py-3 text-gray-400 text-sm">No users found</div>}
						>
							<For each={searchResults()!.users}>
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
					</Suspense>
				</div>
			</Show>
		</div>
	);
};
