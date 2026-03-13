import { A } from "@solidjs/router";
import { createSignal, Show, createResource, Suspense } from "solid-js";
import { FiKey, FiSettings, FiLogOut } from "solid-icons/fi";
import { useAuthState, useClickOutside } from "~/hooks";
import { httpRequest } from "~/utils/http-request";
import CopyableField from "./copyable-field";
import { CopyableFieldVariant } from "~/utils/color";
import Initials from "./initials";
import { useToast } from "~/components/toast";
import { GetUserInfoResponse } from "~/bindings";

interface UserInfo {
	id: string;
	username: string;
	firstName?: string;
	lastName?: string;
	recoveryEmail?: string;
}

const UserDropdown = () => {
	const [isOpen, setIsOpen] = createSignal(false);
	const [authState, setAuthState] = useAuthState();
	const [dropdownRef, setDropdownRef] = createSignal<HTMLDivElement>();
	const toast = useToast();

	useClickOutside(dropdownRef, () => {
		setIsOpen(false);
	});

	const [userInfo] = createResource(authState(), async (auth) => {
		if (auth === null || auth.type !== "LoggedIn") {
			console.log("Auth is null or LoggedOut, returning null");
			return null;
		}

		try {
			const response = await httpRequest<GetUserInfoResponse>(`${import.meta.env.VITE_BASE_URL}/api/user`, {
				method: "GET",
			});

			if (!response.ok) {
				console.error("Failed to fetch workspaces:", response.data.error);
				toast("Failed to fetch workspaces", "error");
				return {
					id: "Unknown",
					username: "unknown",
					firstName: "",
					lastName: "",
					recoveryEmail: "",
				};
			}

			return response.data;
		} catch (error) {
			console.error("Failed to fetch user info:", error);
			return null;
		}
	});

	const handleLogout = () => {
		setAuthState({ type: "LoggedOut" });
		window.location.href = "/login";
	};

	// Add click outside listener with onMount
	const getDisplayName = () => {
		const user = userInfo();
		return user ? `${user.firstName || ""} ${user.lastName || ""}`.trim() || user.username : "User";
	};

	return (
		<div class="relative" ref={setDropdownRef}>
			<Suspense
				fallback={
					<button class="flex items-center gap-2 px-4 py-2 rounded-xs bg-secondary-light hover:bg-secondary-medium transition-colors duration-200 border border-white/10">
						<Initials size="sm" firstName={".."} />
						<span class="text-sm font-medium text-white">User</span>
					</button>
				}
			>
				<button
					onClick={() => {
						setIsOpen(!isOpen());
					}}
					class="flex items-center gap-2 px-4 py-2 rounded-xs bg-secondary-light hover:bg-secondary-medium transition-colors duration-200 border border-white/10 cursor-pointer"
				>
					<Initials firstName={userInfo()?.firstName} lastName={userInfo()?.lastName} size="sm" />
					<span class="text-sm font-medium text-white">{getDisplayName()}</span>
				</button>

				<Show when={isOpen()}>
					<div class="absolute right-0 mt-2 w-80 bg-secondary-medium border border-white/10 rounded-lg shadow-xl overflow-hidden z-50">
						<div class="p-4 border-b border-white/10">
							<div class="flex items-center gap-3 mb-3">
								<Initials firstName={userInfo()?.firstName} lastName={userInfo()?.lastName} size="lg" />
								<div class="flex-1 min-w-0">
									<Show
										when={!userInfo.loading}
										fallback={<div class="text-gray-400 text-sm">Loading...</div>}
									>
										<div class="text-white font-medium truncate">
											{userInfo()?.firstName && userInfo()?.lastName
												? `${userInfo()!.firstName} ${userInfo()!.lastName}`
												: userInfo()?.username || "Unknown User"}
										</div>
										<div class="text-gray-400 text-sm truncate">
											{userInfo()?.recoveryEmail || "No email"}
										</div>
									</Show>
								</div>
							</div>

							<div class="mb-2">
								<CopyableField
									variant={CopyableFieldVariant.Input}
									label="User ID"
									value={userInfo()?.id || ""}
								/>
							</div>

							<CopyableField
								variant={CopyableFieldVariant.Input}
								label="Username"
								value={userInfo()?.username || ""}
							/>
						</div>

						<div class="p-2">
							<A
								href="/profile/api-tokens"
								class="flex items-center gap-3 px-3 py-2 rounded-xs hover:bg-white/5 transition-colors text-gray-300 hover:text-white"
								onClick={() => setIsOpen(false)}
							>
								<FiKey size={16} />
								<span class="text-sm">API Keys</span>
							</A>
							<A
								href="/profile"
								class="flex items-center gap-3 px-3 py-2 rounded-xs hover:bg-white/5 transition-colors text-gray-300 hover:text-white"
								onClick={() => setIsOpen(false)}
							>
								<FiSettings size={16} />
								<span class="text-sm">Account Settings</span>
							</A>
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
			</Suspense>
		</div>
	);
};

export default UserDropdown;
