// @refresh reload
import "solid-devtools";
import { mount, StartClient } from "@solidjs/start/client";
import { attachDevtoolsOverlay } from "@solid-devtools/overlay";
attachDevtoolsOverlay({ defaultOpen: false });

mount(() => <StartClient />, document.getElementById("app")!);
