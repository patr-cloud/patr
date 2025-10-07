import { Route, Router } from "@solidjs/router";
import Login from "./login";

export default function LoggedOutRoutes() {
  return (
    <Router>
      <Route path="/login" component={Login} />
    </Router>
  );
}
