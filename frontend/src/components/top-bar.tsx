import { Component } from "solid-js";
import UserDropdown from "./user-dropdown";

const TopBar: Component = () => {
  return (
    <header class="h-16 bg-secondary border-b border-white/5 flex items-center justify-end px-6">
      <UserDropdown />
    </header>
  );
};

export default TopBar;
