import { Accessor, createContext, useContext } from "solid-js";

export interface SidebarContextValue {
	isMobileOpen: Accessor<boolean>;
	setMobileOpen: (open: boolean) => void;
	toggleMobile: () => void;
}

export const SidebarContext = createContext<SidebarContextValue>();

export const useSidebar = (): SidebarContextValue => {
	const ctx = useContext(SidebarContext);
	if (!ctx) {
		return {
			isMobileOpen: () => false,
			setMobileOpen: () => {},
			toggleMobile: () => {},
		};
	}
	return ctx;
};
