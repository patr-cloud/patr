import { Route, Router } from "@solidjs/router";
import "./app.css";
import LoggedOutRoutes from "./routes/logged-out-routes/index";
import NotFound from "./routes/not_found";
import LoggedInRoutes from "./routes/logged-in-routes";
import Cookies from "js-cookie";
import { createSignal, onMount } from "solid-js";

export default function App() {
  const [isLoggedIn, setIsLoggedIn] = createSignal(false);

  onMount(() => {
    const token = Cookies.get("authToken");
    setIsLoggedIn(!!token);
  });

  return (
    <Router>
      {isLoggedIn() ? <LoggedInRoutes /> : <LoggedOutRoutes />}
      <Route path="*" component={NotFound} />
    </Router>
  );
}
