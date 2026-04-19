# Patr CLI

`patr` is the command-line tool for managing your Patr resources — deployments, runners, container registry, and more — from your terminal.

## Install

One-liner:

```sh
curl -fsSL https://raw.githubusercontent.com/patr-cloud/patr/develop/assets/cli/install.sh | sh
```

Installs to `/usr/local/bin/patr` (prompts for your password). Supports Linux x86_64 / arm64 and macOS arm64.

### Options

Pick a release channel (default: `alpha`):

```sh
curl -fsSL https://raw.githubusercontent.com/patr-cloud/patr/develop/assets/cli/install.sh | sh -s -- --channel beta
```

Install to a user-owned directory (no sudo):

```sh
curl -fsSL https://raw.githubusercontent.com/patr-cloud/patr/develop/assets/cli/install.sh | sh -s -- --prefix $HOME/.local/bin
```

The script refuses to overwrite an existing install — use `patr upgrade` to update or `patr uninstall` to reinstall from scratch.

## Keeping it up to date

```sh
patr upgrade
```

Tracks the channel you installed on. Switch channels with `patr upgrade --channel stable|beta|alpha`; the choice is persisted. Use `--check` to see what would happen without modifying anything.

## Uninstall

```sh
patr uninstall
```

Removes the CLI config, any installed systemd runner services, and the binary itself. Pass `--purge` to also tear down resources managed by Patr runners on this host (`docker swarm leave --force`).

## Getting started

```sh
patr login             # authenticate with your Patr account
patr info              # show the current user
patr deployments       # list deployments in the current workspace
patr --help            # full command reference
```
