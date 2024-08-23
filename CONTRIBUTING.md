# Contributing to Patr

Firstly, thank you for your interest in contributing to Patr! Contributions are greatly appreciated.

Participation is governed by the [Code of Conduct](./CODE_OF_CONDUCT.md). By contributing, you agree to the following terms:

- **Licensing of Contributions:** All contributions to this project are automatically licensed under the GNU Affero General Public License (AGPL) for open-source use.
- **Future Relicensing:** By submitting contributions to this project, you agree that the owner of this repository has the perpetual, worldwide right to relicense your contributions under any other license terms in the future, while retaining an open source license for open-source use. This means the owner of this repository can, at any point, relicense the project, including your contributions, under different commercial terms, but the AGPL will always apply for the open-source community under any circumstances.

## How to contribute

In order to prevent people's efforts from going to waste, we ask that you communicate with us before starting work on a new feature or a significant change. This way, we can ensure that your work is not duplicating someone else's efforts. Creating GitHub issues and getting them assigned to you is a good way to track who is working on what.

We would be happy to mentor you on the codebase if you need any assistance!

## Pull requests

Please run rustfmt on your codebase before submitting a PR. This will ensure that the codebase is consistent and easy to read.

You can run rustfmt by running:

```bash
cargo +nightly fmt
```

Oh, also - we use tabs instead of spaces. Controversial, I know. But I don't find it productive to have conversations about tabs vs spaces. You're free to setup your editor to automatically convert tabs to spaces and convert them back when committing, but please don't submit PRs that change the indentation style.

## Code structure

For details on the architecture of the project, please refer to the [ARCHITECTURE.md](./ARCHITECTURE.md) file.
