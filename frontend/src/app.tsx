import { Route, Router } from "@solidjs/router";
import "./app.css";
import LoggedOutRoutes from "./routes/logged-out-routes/index";
import NotFound from "./routes/not-found";
import LoggedInRoutes from "./routes/logged-in-routes";
import { useAuthState } from "./utils/state";

export default function App() {
  const [authState, _] = useAuthState();

  console.log("Auth State:", authState());

  return (
    <Router>
      <LoggedInRoutes />
      <LoggedOutRoutes />
      <Route path="*" component={NotFound} />
    </Router>
  );
}
