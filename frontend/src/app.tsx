import { Route, Router } from "@solidjs/router";
import { createSignal, Suspense } from "solid-js";
import "./app.css";
import Login from "./routes/login";
import LoggedOutRoutes from "./routes/logged-out-routes";

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
      {loggedIn() ? (
        <Route path="/" component={() => <div>Home</div>} />
      ) : (
        <LoggedOutRoutes />
      )}
    </Router>
  );
}
