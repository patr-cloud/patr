// @refresh reload
import { createHandler, StartServer } from "@solidjs/start/server";

export default createHandler(() => (
	<StartServer
		document={({ assets, children, scripts }) => (
			<html lang="en">
				<head>
					<meta charset="utf-8" />
					<meta name="viewport" content="width=device-width, initial-scale=1" />
					<link rel="icon" href="/favicon.ico" />
					{assets}
				</head>
				<body>
					<div id="app">{children}</div>
					{scripts}
					{/* eslint-disable-next-line solid/self-closing-comp -- <script> is not a void element; self-closing breaks HTML */}
					<script src="https://challenges.cloudflare.com/turnstile/v0/api.js" async defer></script>
				</body>
			</html>
		)}
	/>
));
