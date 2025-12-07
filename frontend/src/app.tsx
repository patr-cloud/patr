// import { clientOnly } from "@solidjs/start";
import { Route, Router } from "@solidjs/router";
import "./app.css";
import LoggedOutRoutes from "./routes/logged-out-routes/index";
import NotFound from "./routes/not-found";
import LoggedInRoutes from "./routes/logged-in-routes";
import {
	AuthStateProvider,
	LastWorkspaceIdProvider,
} from "~/hooks/state-hooks";
import { ToastProvider } from "./components";

function App() {
	return (
		<AuthStateProvider>
			<LastWorkspaceIdProvider>
				<ToastProvider>
					<Router>
						<LoggedOutRoutes />
						<LoggedInRoutes />
						<Route path="*" component={NotFound} />
					</Router>
				</ToastProvider>
			</LastWorkspaceIdProvider>
		</AuthStateProvider>
	);
}

// export default clientOnly(async () => ({ default: App }), { lazy: true });
export default App;
