# Patr

<center>
    <a href="https://deepwiki.com/patr-cloud/patr"><img src="https://deepwiki.com/badge.svg" alt="Ask DeepWiki"></a>
    <img alt="GitHub top language" src="https://img.shields.io/github/languages/top/patr-cloud/patr">
</center>

**Patr** is a self-hosted, production-grade DevOps automation platform built for fast-moving teams.

It lets developers deploy and manage services without needing to understand or operate traditional DevOps infrastructure. No Kubernetes expertise required. No bespoke CI/CD glue. No hand-rolled observability stacks.

Patr is designed to feel closer to a _control plane_ than a hosting provider.

> ⚠️ **Early Beta**  
> Patr is still in the early stages of development. APIs and resource models may change.  
> Not recommended for mission-critical production workloads yet.
> We would LOVE to have contributors onboard, but please be aware that the project is not yet ready for production use.

# Using the CLI

To manage Patr resources from your terminal, install the CLI:

```sh
curl -fsSL https://raw.githubusercontent.com/patr-cloud/patr/develop/assets/cli/install.sh | sh
```

See [cli/README.md](./cli/README.md) for channels, upgrade, and uninstall instructions.

# Running the project

Nobody reads large text anyway (other than LLMs), so here's a skimmed down version:

### Dependencies needed

- [Rust](https://www.rust-lang.org/tools/install): `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- [cargo leptos](https://github.com/leptos-rs/cargo-leptos): `cargo install cargo-leptos`
- Postgres
- Redis

### Create a config file

Copy `./config/api.sample.json` to `./config/api.json` and fill in the required fields. You'll primarily need to fill in `database` and `redis` fields.

### Run the project

```bash
cargo api
```

### ???

### Profit!

You can now access the project at `http://localhost:3001`.

# Features

Patr is built with Rust, Axum, and Postgres. It is designed to be fast, secure, and scalable. Your code can be deployed to any environment from a single dashboard. For example, you can manage your deployments that are running on a VM (say, EC2 instance) as well as your deployments that are running on a Kubernetes cluster, all from a simple, easy-to-use, unified dashboard. Where your code runs is up to you, Patr just helps you manage it. This means that you can also use Patr to manage your deployments on your local machine, if you so desire. A common use-case of having a home-box on a Raspberry Pi, for example, and managing your deployments on it with Patr is supported.

# Contributing

We'd love to have you onboard! Please read the [ARCHITECTURE.md](./ARCHITECTURE.md) file for more information on how to get started and the [CONTRIBUTING.md](./CONTRIBUTING.md) file for details on how to contribute.
