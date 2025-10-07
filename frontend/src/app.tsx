import { Route, Router } from "@solidjs/router";
import { createSignal, Suspense } from "solid-js";
import "./app.css";
import LoggedOutRoutes from "./routes/logged-out-routes/index";
import NotFound from "./routes/not_found";

export default function App() {
  const [loggedIn, _] = createSignal(false);

  return (
    <Router
      root={(props) => (
        <>
          <Suspense>{props.children}</Suspense>
        </>
      )}
    >
      <LoggedOutRoutes />
      <Route path="*" component={NotFound} />
    </Router>
  );
}
