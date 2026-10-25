// @refresh reload
import "solid-devtools";
import { mount, StartClient } from "@solidjs/start/client";
import { attachDevtoolsOverlay } from "@solid-devtools/overlay";
attachDevtoolsOverlay({ defaultOpen: false });

export default function mountApp() {
	mount(() => <StartClient />, document.getElementById("app")!);
}

if (import.meta.env.VITE_BUILD_TARGET !== "csr") {
	mountApp();
}
