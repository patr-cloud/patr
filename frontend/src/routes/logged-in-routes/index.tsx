import { Route } from "@solidjs/router";

export default function LoggedInRoutes() {
  return (
    <>
      <Route path="/" component={() => <div>Home</div>} />
    </>
  );
}
