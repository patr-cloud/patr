// @refresh reload
import { mount, StartClient } from "@solidjs/start/client";

export default function startClient() {
	mount(() => <StartClient />, document.getElementById("app")!);
}
