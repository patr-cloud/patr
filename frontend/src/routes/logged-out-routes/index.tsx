import { Route } from "@solidjs/router";
import { Navigate } from "@solidjs/router";
import Login from "./login";
import SignUp from "./sign-up";
import ForgotPassword from "./forgot-password";

export default function LoggedOutRoutes() {
  return (
    <>
      <Route path="/" component={() => <Navigate href="/login" />} />
      <Route path="/login" component={Login} />
      <Route path="/sign-up" component={SignUp} />
      <Route path="/forgot-password" component={ForgotPassword} />
      <Route path="*" component={() => <Navigate href="/login" />} />
    </>
  );
}
