# Patr - Open Source, AI powered, DevOps platform

Patr is a tool that helps you deploy your applications to multiple environments with ease. It is designed to be simple to use and easy to integrate with your existing CI/CD pipelines.

# WORK IN PROGRESS

This project is still in the early stages of development.

Would LOVE to have contributors onboard, but please be aware that the project is not yet ready for production use.

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
cargo leptos serve
```

### ???

### Profit!

You can now access the project at `http://localhost:3000`.

# Features

Patr is built with Rust, Axum, and Postgres. It is designed to be fast, secure, and scalable. Your code can be deployed to any environment from a single dashboard. For example, you can manage your deployments that are running on a VM (say, EC2 instance) as well as your deployments that are running on a Kubernetes cluster, all from a simple, easy-to-use, unified dashboard. Where your code runs is up to you, Patr just helps you manage it. This means that you can also use Patr to manage your deployments on your local machine, if you so desire. A common use-case of having a home-box on a Raspberry Pi, for example, and managing your deployments on it with Patr is supported.

Here are some of the features that we plan to implement:

- ✅ Create and manage deployments.
- ✅ Create and manage environment variables.
- ✅ Create and manage Managed URLs.
- ✅ Create runners to run your deployments.
- ✅ Create and manage users.
- ✅ Create and manage roles.
- 🚧 Implement the runner to manage the deployments on the server.
- 🚧 Audit log for all actions that are performed on Patr. (Help needed: What's the data that needs to be stored and how do I store it?)
- [ ] Create and manage secrets.
- [ ] Create and manage databases.


# Contributing

We'd love to have you onboard! Please read the [ARCHITECTURE.md](./ARCHITECTURE.md) file for more information on how to get started.
