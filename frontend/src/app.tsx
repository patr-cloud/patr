import "./app.css";
import { RouterProvider } from "@tanstack/solid-router";
import { MetaProvider } from "@solidjs/meta";
import { QueryCache, QueryClient, QueryClientProvider } from "@tanstack/solid-query";
import { createAppRouter } from "./router";
import { AuthStateProvider, LastWorkspaceIdProvider, useAuthState } from "~/hooks/state-hooks";
import { ToastProvider, useToast } from "./components";
import { isServer } from "solid-js/web";
import { ParentProps } from "solid-js";

// Singleton on the client; server creates per-request
const clientRouter = isServer ? null : createAppRouter();

function InnerApp() {
	const [authState] = useAuthState();
	const router = clientRouter ?? createAppRouter();
	return <RouterProvider router={router} context={{ auth: authState() }} />;
}

function createQueryClient(toast: (msg: string, level: "warn" | "error" | "success" | "info") => void) {
	return new QueryClient({
		queryCache: new QueryCache({
			onError: (_error, query) => {
				const message = query.meta?.errorMessage;
				if (typeof message === "string") {
					toast(message, "error");
				}
			},
		}),
	});
}

let clientQueryClient: QueryClient | null = null;

function QueryLayer(props: ParentProps) {
	const toast = useToast();
	if (!clientQueryClient && !isServer) {
		clientQueryClient = createQueryClient(toast);
	}
	const queryClient = clientQueryClient ?? createQueryClient(toast);

	return <QueryClientProvider client={queryClient}>{props.children}</QueryClientProvider>;
}

function App() {
	return (
		<MetaProvider>
			<ToastProvider>
				<QueryLayer>
					<AuthStateProvider>
						<LastWorkspaceIdProvider>
							<InnerApp />
						</LastWorkspaceIdProvider>
					</AuthStateProvider>
				</QueryLayer>
			</ToastProvider>
		</MetaProvider>
	);
}

export default App;
