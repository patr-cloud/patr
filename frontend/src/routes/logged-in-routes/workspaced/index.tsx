import { Route } from "@solidjs/router";

export default function WorkspacedRoutes() {
  return (
    <>
      <Route path="/" component={() => <div>Home</div>} />
    </>
  );
}
