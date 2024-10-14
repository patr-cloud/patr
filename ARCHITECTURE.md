# Git hiker's guide to the galaxy

Thank you so much for your interest in contributing to Patr! This document is a guide to help you understand the architecture of the project, and how you can contribute to it.

Participation is governed by the Code of Conduct and the Licensing Terms as mentioned in [CONTRIBUTING.md](./CONTRIBUTING.md).

## Contribution instructions, again

Since most people wouldn't follow along with the instructions in the CONTRIBUTING.md file, here it is again.

<sub>Psst....jump to [Development environment](#development-environment) to skip ahead.<sub>

### How to contribute

In order to prevent people's efforts from going to waste, we ask that you communicate with us before starting work on a new feature or a significant change. This way, we can ensure that your work is not duplicating someone else's efforts. Creating GitHub issues and getting them assigned to you is a good way to track who is working on what.

We would be happy to mentor you on the codebase if you need any assistance!

### Pull requests

Please run rustfmt on your codebase before submitting a PR. This will ensure that the codebase is consistent and easy to read.

You can run rustfmt by running:

```bash
cargo +nightly fmt
```

Oh, also - we use tabs instead of spaces. Controversial, I know. But I don't find it productive to have conversations about tabs vs spaces. You're free to setup your editor to automatically convert tabs to spaces and convert them back when committing, but please don't submit PRs that change the indentation style.

## Development environment

I have setup a DevContainer that covers all the dependencies and tools you need to contribute to this project. Simply open this project in VSCode (or any other editor that supports DevContainers) and you will be prompted to open the project in a DevContainer. In all fairness, I have only set this up for myself, so if you run into any issues, please let me know.

Once a DevContainer is running, create a config file by copying the `./config/api.sample.json` file to `./config/api.json` and fill in the required fields (the defaults will work perfectly fine for the DevContainer). You'll primarily need to fill in `database` and `redis` fields if you're not using a DevContainer.

Once setup, you can run the project by running:

```bash
cargo leptos serve
```

You can now access the project at `http://localhost:3000`.

## Onto the good stuff!

First, some terminology.

- **Deployment**: A deployment is the configuration for your code running on a server. This includes the environment variables, the docker image that it is running, where the code is running (called a runner), and all other configurations that are needed to run your image.
- **Database**: The database that is used by the API. This can be Postgres, MySQL, SQLite, etc.
- **Secrets**: Secrets are sensitive information that is needed by the deployment. This can be a password, an API key, etc.
- **Static site**: A site that is hosted on Patr. This can be a React app, a Vue app, a Svelte app, etc.
- **Managed URL**: This is essentially a URL that Patr helps manage for you. This URL can point to a deployment, a static site or redirect to something else entirely. Imagine this to be like an NGINX configuration that you can manage from a UI.
- **Runner**: A runner is the service that is responsible for running your deployment, database and your Managed URL. This can be a Kubernetes cluster, a VM, a Raspberry Pi, or even your local machine.

Okay, here's a rough architecture of the project. The whole repo is a single mono-repo workspace that contains the following:

- `./api`: The backend API that hosts the API as well as the frontend.
- `./cli`: The CLI that interacts with the API.
- `./components`: The shared Leptos components that are used by the frontend.
- `./config`: The configuration files for the API.
- `./frontend`: The frontend for Patr.
- `./macros`: Commonly used macros for the project.
- `./models`: The models that are shared throughout the codebase. This includes:
  - `./api`: The format for request, response, error, headers and query parameters.
  - `./cloudflare`: The format of data that is stored in Cloudflare KV.
  - `./iaac`: The format of data that will be used by IaaC files.
  - `./rbac`: The list of resource types, permissions and the way they are stored.
  - `./utils`: Commonly used utilities.
- `./runners`: The runners that are used to run the deployments.

## How runners work

Each runner is basically a piece of code that connects to the API via websocket, and listens for changes. When a deployment is created, the API notifies the runner about the deployment. The runner then pulls the docker image, creates the container, and starts the container. The runner then listens for changes to the deployment and updates the container accordingly. It also listens for changes to the container to update the status of the deployment (running, stopped, etc).

Patr comes built-in with two runners: Docker runner and Kubernetes runner (the code for which can be found in the `./runners` directory). The docker runner can be used to run your deployments on a VM, Raspberry Pi, etc.

The `./runners/common` is used to share code between the runners. This includes the code to connect to the API, listen for changes, and update the status of the deployment. It also manages the storage of the deployment data, and periodically syncing the data with the API. The common library is still a work in progress, and is not yet complete. This would provide a single trait with async fns that can be used to implement a runner. All the syncing and updating of the deployment data would be handled by the common library.

Fun fact - since runners are essentially just pieces of code that connect to the API, you can write your own runner to run your deployments on your own infrastructure. The API is designed to be agnostic to where the code is running, so you can even use Patr to manage your deployments across multiple PaaS providers!

## How routes are declared

This is a completely in-house system that was developed in order to avoid invalid responses. Essentially, there is a singular `ApiEndpoint` trait [here](./models/src/endpoint.rs), that declares an associated type for the request body, response body, request headers, response headers, query parameters and path parameters. This trait is then implemented for each route in the API. This allows us to have a single source of truth for the route, and ensures that the response is always valid. Additionally, response headers and request headers are implemented as structs. Trait bounds are used to ensure that the required headers are always present.

There's a lot of things going on here, but for the most part, you don't need to worry about this. The `declare_api_endpoint!` macro (documented [here](./macros/src/lib.rs)) does all the heavy lifting for you.
