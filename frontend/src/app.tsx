import { Route, Router } from "@solidjs/router";
import "./app.css";
import LoggedOutRoutes from "./routes/logged-out-routes/index";
import NotFound from "./routes/not-found";
import LoggedInRoutes from "./routes/logged-in-routes";
import {
  AuthStateProvider,
  LastWorkspaceIdProvider,
} from "~/hooks/state-hooks";

export default function App() {
  return (
    <AuthStateProvider>
      <LastWorkspaceIdProvider>
        <Router>
          <LoggedInRoutes />
          <LoggedOutRoutes />
          <Route path="*" component={NotFound} />
        </Router>
      </LastWorkspaceIdProvider>
    </AuthStateProvider>
  );
}
