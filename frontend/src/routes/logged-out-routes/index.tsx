import { Route } from "@solidjs/router";
import { Navigate } from "@solidjs/router";
import Login from "./login";
import SignUp from "./sign-up";
import ConfirmSignup from "./confirm-signup";
import ForgotPassword from "./forgot-password";
import AuthLayout from "../../pages/auth-layout";

export default function LoggedOutRoutes() {
  return (
    <>
      <Route path="/" component={() => <Navigate href="/login" />} />
      <Route path="/login" component={() => <AuthLayout><Login /></AuthLayout>} />
      <Route path="/sign-up" component={() => <AuthLayout><SignUp /></AuthLayout>} />
      <Route path="/confirm-signup" component={() => <AuthLayout><ConfirmSignup /></AuthLayout>} />
      <Route path="/forgot-password" component={() => <AuthLayout><ForgotPassword /></AuthLayout>} />
    </>
  );
}
