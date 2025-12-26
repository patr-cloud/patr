import "./app.css";
import { Route, Router } from "@solidjs/router";
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

export default App;
