import { Link } from "@tanstack/solid-router";
import { createSignal, Show } from "solid-js";
import { FiKey, FiSettings, FiLogOut } from "solid-icons/fi";
import { useAuthState, useClickOutside } from "~/hooks";
import CopyableField from "./copyable-field";
import { CopyableFieldVariant } from "~/utils/color";
import Initials from "./initials";
import { useUserInfoQuery } from "~/hooks/fetch";

const UserDropdown = () => {
	const [isOpen, setIsOpen] = createSignal(false);
	const [_authState, setAuthState] = useAuthState();
	const [dropdownRef, setDropdownRef] = createSignal<HTMLDivElement>();

	useClickOutside(dropdownRef, () => {
		setIsOpen(false);
	});

	const userInfoQuery = useUserInfoQuery();

	const handleLogout = () => {
		setAuthState({ type: "LoggedOut" });
		window.location.href = "/login";
	};

	const getDisplayName = () => {
		const user = userInfoQuery.data;
		return user ? `${user.firstName || ""} ${user.lastName || ""}`.trim() || user.username : "User";
	};

	return (
		<div class="relative" ref={setDropdownRef}>
			<button
				onClick={() => {
					setIsOpen(!isOpen());
				}}
				class="flex items-center gap-2 px-4 py-2 rounded-xs bg-secondary-light hover:bg-secondary-medium transition-colors duration-200 border border-white/10 cursor-pointer"
			>
				<Initials firstName={userInfoQuery.data?.firstName} lastName={userInfoQuery.data?.lastName} size="sm" />
				<span class="text-sm font-medium text-white">{getDisplayName()}</span>
			</button>

			<Show when={isOpen()}>
				<div class="absolute right-0 mt-2 w-80 bg-secondary-medium border border-white/10 rounded-lg shadow-xl overflow-hidden z-50">
					<div class="p-4 border-b border-white/10">
						<div class="flex items-center gap-3 mb-3">
							<Initials
								firstName={userInfoQuery.data?.firstName}
								lastName={userInfoQuery.data?.lastName}
								size="lg"
							/>
							<div class="flex-1 min-w-0">
								<div class="text-white font-medium truncate">
									{userInfoQuery.data?.firstName && userInfoQuery.data?.lastName
										? `${userInfoQuery.data!.firstName} ${userInfoQuery.data!.lastName}`
										: userInfoQuery.data?.username || "Unknown User"}
								</div>
								<div class="text-gray-400 text-sm truncate">
									{userInfoQuery.data?.recoveryEmail || "No email"}
								</div>
							</div>
						</div>

						<div class="mb-2">
							<CopyableField
								variant={CopyableFieldVariant.Input}
								label="User ID"
								value={userInfoQuery.data?.id || ""}
							/>
						</div>

						<CopyableField
							variant={CopyableFieldVariant.Input}
							label="Username"
							value={userInfoQuery.data?.username || ""}
						/>
					</div>

					<div class="p-2">
						<Link
							to="/profile/api-tokens"
							class="flex items-center gap-3 px-3 py-2 rounded-xs hover:bg-white/5 transition-colors text-gray-300 hover:text-white"
							onClick={() => setIsOpen(false)}
						>
							<FiKey size={16} />
							<span class="text-sm">API Keys</span>
						</Link>
						<Link
							to={"/profile" as string}
							class="flex items-center gap-3 px-3 py-2 rounded-xs hover:bg-white/5 transition-colors text-gray-300 hover:text-white"
							onClick={() => setIsOpen(false)}
						>
							<FiSettings size={16} />
							<span class="text-sm">Account Settings</span>
						</Link>
					</div>

					<div class="p-2 border-t border-white/10">
						<button
							onClick={handleLogout}
							class="w-full flex items-center gap-3 px-3 py-2 rounded-xs hover:bg-red-500/10 transition-colors text-gray-300 hover:text-red-400"
						>
							<FiLogOut size={16} />
							<span class="text-sm">Logout</span>
						</button>
					</div>
				</div>
			</Show>
		</div>
	);
};

export default UserDropdown;
