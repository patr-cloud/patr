import "./app.css";
import { RouterProvider } from "@tanstack/solid-router";
import { MetaProvider } from "@solidjs/meta";
import { createAppRouter } from "./router";
import { AuthStateProvider, LastWorkspaceIdProvider, useAuthState } from "~/hooks/state-hooks";
import { ToastProvider } from "./components";
import { isServer } from "solid-js/web";

// Singleton on the client; server creates per-request in InnerApp
const clientRouter = isServer ? null : createAppRouter();

function InnerApp() {
	const [authState] = useAuthState();
	const router = clientRouter ?? createAppRouter();
	return <RouterProvider router={router} context={{ auth: authState() }} />;
}

function App() {
	return (
		<MetaProvider>
			<AuthStateProvider>
				<LastWorkspaceIdProvider>
					<ToastProvider>
						<InnerApp />
					</ToastProvider>
				</LastWorkspaceIdProvider>
			</AuthStateProvider>
		</MetaProvider>
	);
}

export default App;
