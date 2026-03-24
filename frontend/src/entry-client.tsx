// @refresh reload
import { mount, StartClient } from "@solidjs/start/client";

export default function mountApp() {
	mount(() => <StartClient />, document.getElementById("app")!);
}

if (import.meta.env.VITE_BUILD_TARGET !== "csr") {
	mountApp();
}
