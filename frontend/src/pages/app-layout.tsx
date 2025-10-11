import { JSX, createSignal } from "solid-js";
import Button from "~/components/button";
import { ButtonVariant } from "~/utils/color";

interface AppLayoutProps {
  children: JSX.Element;
}

const AppLayout = (props: AppLayoutProps) => {
  const [isUserMenuOpen, setIsUserMenuOpen] = createSignal(false);

  return (
    <div class="min-h-screen w-full bg-secondary flex flex-col">
      {/* Header */}
      <header class="bg-secondary-dark border-b border-secondary-medium px-6 py-4">
        <div class="flex items-center justify-between">
          {/* Left side - Logo and future navigation */}
          <div class="flex items-center space-x-8">
            {/* Patr Logo */}
            <div class="flex items-center">
              <img
                src="/images/patr.svg"
                alt="Patr Logo"
                class="h-8 w-auto"
              />
            </div>
            
            {/* Future navigation sidebar toggle would go here */}
            <div class="hidden md:flex items-center space-x-6">
              {/* Navigation items will be added in future iterations */}
            </div>
          </div>

          {/* Right side - User menu */}
          <div class="flex items-center space-x-4">
            {/* User menu dropdown */}
            <div class="relative">
              <Button
                variant={ButtonVariant.Plain}
                class="flex items-center space-x-2 text-white hover:text-primary"
                onClick={() => setIsUserMenuOpen(!isUserMenuOpen())}
              >
                {/* User avatar placeholder */}
                <div class="w-8 h-8 bg-primary rounded-full flex items-center justify-center">
                  <span class="text-secondary text-sm font-medium">U</span>
                </div>
                {/* Dropdown arrow */}
                <svg
                  class={`w-4 h-4 transition-transform ${
                    isUserMenuOpen() ? "rotate-180" : ""
                  }`}
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M19 9l-7 7-7-7"
                  />
                </svg>
              </Button>

              {/* Dropdown menu */}
              {isUserMenuOpen() && (
                <div class="absolute right-0 mt-2 w-48 bg-secondary-dark border border-secondary-medium rounded-md shadow-lg z-50">
                  <div class="py-1">
                    <button class="block w-full text-left px-4 py-2 text-sm text-white hover:bg-secondary-medium">
                      Profile
                    </button>
                    <button class="block w-full text-left px-4 py-2 text-sm text-white hover:bg-secondary-medium">
                      Settings
                    </button>
                    <hr class="border-secondary-medium my-1" />
                    <button class="block w-full text-left px-4 py-2 text-sm text-white hover:bg-secondary-medium">
                      Logout
                    </button>
                  </div>
                </div>
              )}
            </div>
          </div>
        </div>
      </header>

      {/* Main content area */}
      <main class="flex-1 flex">
        {/* Future navigation sidebar space */}
        <div class="hidden lg:block w-64 bg-secondary-light border-r border-secondary-medium">
          {/* Navigation sidebar will be implemented in future iterations */}
          <div class="p-4">
            <div class="text-gray-400 text-sm">Navigation</div>
            <div class="mt-4 space-y-2">
              {/* Placeholder for future navigation items */}
              <div class="text-gray-500 text-xs">• Workspaces</div>
              <div class="text-gray-500 text-xs">• Deployments</div>
              <div class="text-gray-500 text-xs">• Settings</div>
            </div>
          </div>
        </div>

        {/* Content area with consistent padding */}
        <div class="flex-1 p-6 overflow-auto">
          {props.children}
        </div>
      </main>
    </div>
  );
};

export default AppLayout;