## Build instructions

Everything runs inside a devcontainer. Just open in vscode, and select "open in devcontainer". The init scripts will take care of the rest.

The scripts are located in `.devcontainer/scripts`.

If you're looking to run `cargo sqlx prepare`, there's a shortcut `cargo prepare` which includes all the right credentials.

The bash alias `start-frontend` can be used to start the frontend automatically on port 3001.

The backend is automatically accessible on `https://api.patr.cloud` within the devcontainer, and the registry is available at `https://registry.patr.cloud` within the devcontainer. HTTPS trust is all automatically setup. No config required.
