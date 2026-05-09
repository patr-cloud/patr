# Contributing to Patr

Firstly, thank you for your interest in contributing to Patr! Contributions are greatly appreciated.

Participation is governed by the [Code of Conduct](./CODE_OF_CONDUCT.md). By submitting a contribution to this project, you agree to the following terms:

- **License of contribution:** Your contribution is licensed to the project under the license specified in [LICENSE](./LICENSE).
- **Future relicensing:** You grant the owner of this repository a perpetual, worldwide, irrevocable, royalty-free right to relicense your contribution under additional or different terms, including commercial or proprietary terms.
- **Patent grant:** You grant a perpetual, worldwide, irrevocable, royalty-free patent license to the project and its users covering any patents you hold that are necessarily infringed by your contribution.
- **Authority and originality:** You confirm that your contribution is your own work, or that you have the legal right to submit it under these terms (for example, your employer has authorised it). If it includes third-party code, you have flagged it clearly in the pull request.
- **No warranty:** Your contribution is provided "AS IS", without warranty of any kind. You will not be liable for damages arising from your contribution.

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
