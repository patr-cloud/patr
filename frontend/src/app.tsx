import { Route, Router } from "@solidjs/router";
import "./app.css";
import LoggedOutRoutes from "./routes/logged-out-routes/index";
import NotFound from "./routes/not_found";
import LoggedInRoutes from "./routes/logged-in-routes";
import { createServerCookie } from "@solid-primitives/cookies";

export default function App() {
  const [authState, _] = createServerCookie("authState");

  return (
    <Router>
      {authState() ? <LoggedInRoutes /> : <LoggedOutRoutes />}
      <Route path="*" component={NotFound} />
    </Router>
  );
}
