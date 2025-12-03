import { A } from "@solidjs/router";
import {
  Component,
  createSignal,
  Show,
  onCleanup,
  createResource,
  onMount,
} from "solid-js";
import { FiUser, FiKey, FiSettings, FiLogOut } from "solid-icons/fi";
import { useAuthState } from "~/hooks";
import { doFetch } from "~/utils/do-fetch";
import CopyableTextField from "./copyable-text-field";

interface UserInfo {
  id: string;
  username: string;
  firstName?: string;
  lastName?: string;
  email?: string;
}

const UserDropdown: Component = () => {
  const [isOpen, setIsOpen] = createSignal(false);
  const [authState, setAuthState] = useAuthState();
  let dropdownRef: HTMLDivElement | undefined;

  // Fetch user info
  const [userInfo] = createResource(authState, async (auth) => {
    console.log("createResource triggered with auth:", auth);
    if (auth === null || auth.type !== "LoggedIn") {
      console.log("Auth is null or LoggedOut, returning null");
      return null;
    }
    try {
      console.log("Fetching user info...");
      const response = await doFetch<UserInfo>(
        `${import.meta.env.VITE_BASE_URL}/api/user`,
        {
          method: "GET",
          headers: {
            "Content-Type": "application/json",
            Authorization: `Bearer ${auth.accessToken}`,
          },
        }
      );
      console.log("User info response:", response.data);
      return response.data;
    } catch (error) {
      console.error("Failed to fetch user info:", error);
      return null;
    }
  });

  console.log("UserDropdown render - userInfo:", userInfo(), "loading:", userInfo.loading, "error:", userInfo.error);

  const handleLogout = () => {
    setAuthState({ type: "LoggedOut" });
    window.location.href = "/login";
  };

  // Add click outside listener with onMount
  onMount(() => {
    console.log("UserDropdown mounted");
    const handleClick = (e: MouseEvent) => {
      if (dropdownRef && !dropdownRef.contains(e.target as Node)) {
        setIsOpen(false);
      }
    };
    
    document.addEventListener("click", handleClick);
    onCleanup(() => {
      document.removeEventListener("click", handleClick);
    });
  });

  const user = userInfo();
  const displayName = user
    ? `${user.firstName || ""} ${user.lastName || ""}`.trim() || user.username
    : "User";

  return (
    <div class="relative" ref={dropdownRef}>
      <button
        onClick={() => {
          console.log("User button clicked, current isOpen:", isOpen());
          setIsOpen(!isOpen());
        }}
        class="flex items-center gap-2 px-4 py-2 rounded-xs bg-secondary-light hover:bg-secondary-medium transition-colors duration-200 border border-white/10"
      >
        <div class="w-8 h-8 rounded-full bg-primary/20 flex items-center justify-center text-primary">
          <FiUser />
        </div>
        <span class="text-sm font-medium text-white">{displayName}</span>
      </button>

      <Show when={isOpen()}>
        <div class="absolute right-0 mt-2 w-80 bg-secondary-medium border border-white/10 rounded-lg shadow-xl overflow-hidden z-50">
          <div class="p-4 border-b border-white/10">
            <div class="flex items-center gap-3 mb-3">
              <div class="w-12 h-12 rounded-full bg-primary/20 flex items-center justify-center text-primary text-lg">
                <FiUser />
              </div>
              <div class="flex-1 min-w-0">
                <Show
                  when={!userInfo.loading}
                  fallback={<div class="text-gray-400 text-sm">Loading...</div>}
                >
                  <div class="text-white font-medium truncate">
                    {user?.firstName && user?.lastName
                      ? `${user.firstName} ${user.lastName}`
                      : user?.username || "Unknown User"}
                  </div>
                  <div class="text-gray-400 text-sm truncate">
                    {user?.email || "No email"}
                  </div>
                </Show>
              </div>
            </div>

            <div class="mb-2">
              <CopyableTextField
                label="User ID"
                value={user?.id || ""}
                disabled={!user?.id}
              />
            </div>

            <CopyableTextField
              label="Username"
              value={user?.username || ""}
              disabled={!user?.username}
            />
          </div>

          <div class="p-2">
            <A
              href="/api-keys"
              class="flex items-center gap-3 px-3 py-2 rounded-lg hover:bg-white/5 transition-colors text-gray-300 hover:text-white"
              onClick={() => setIsOpen(false)}
            >
              <FiKey size={16} />
              <span class="text-sm">API Keys</span>
            </A>
            <A
              href="/user-settings"
              class="flex items-center gap-3 px-3 py-2 rounded-lg hover:bg-white/5 transition-colors text-gray-300 hover:text-white"
              onClick={() => setIsOpen(false)}
            >
              <FiSettings size={16} />
              <span class="text-sm">User Settings</span>
            </A>
          </div>

          <div class="p-2 border-t border-white/10">
            <button
              onClick={handleLogout}
              class="w-full flex items-center gap-3 px-3 py-2 rounded-lg hover:bg-red-500/10 transition-colors text-gray-300 hover:text-red-400"
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
