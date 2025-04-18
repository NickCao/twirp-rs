## Contributing

[fork]: https://github.com/github/twirp-rs/fork
[pr]: https://github.com/github/twirp-rs/compare
[code-of-conduct]: CODE_OF_CONDUCT.md

Hi there! We're thrilled that you'd like to contribute to this project. Your help is essential for keeping it great.

Contributions to this project are [released](https://help.github.com/articles/github-terms-of-service/#6-contributions-under-repository-license) to the public under the [project's open source license](LICENSE.md).

Please note that this project is released with a [Contributor Code of Conduct](CODE_OF_CONDUCT.md). By participating in this project you agree to abide by its terms.

## Prerequisites for running and testing code

We recommend that you install Rust with the `rustup` tool. `twirp-rs` targets stable Rust versions.

## Submitting a pull request

1. [Fork][fork] and clone the repository.
1. Install `protoc` with your package manager of choice.
1. Build the software: `cargo build`.
1. Create a new branch: `git checkout -b my-branch-name`.
1. Make your change, add tests, and make sure the tests and linter still pass.
1. Push to your fork and [submit a pull request][pr].
1. Pat yourself on the back and wait for your pull request to be reviewed and merged.

Here are a few things you can do that will increase the likelihood of your pull request being accepted:

- Write tests.
- Keep your change as focused as possible. If there are multiple changes you would like to make that are not dependent upon each other, consider submitting them as separate pull requests.
- Write a [good commit message](http://tbaggery.com/2008/04/19/a-note-about-git-commit-messages.html).

## Setting up a local build

Make sure you have [rust toolchain installed](https://www.rust-lang.org/tools/install) on your system and then:

```sh
cargo build && cargo test
```

Run clippy and fix any lints:

```sh
make lint
```

## Releasing

1. Go to the `Create Release PR` action and press the button to run the action. This will use `release-plz` to create a new release PR.
1. Adjust the generated changelog and version number(s) as necessary.
1. Get PR approval
1. Merge the PR. The `publish-release.yml` workflow will automatically publish a new release of any crate whose version has changed.

## Resources

- [How to Contribute to Open Source](https://opensource.guide/how-to-contribute/)
- [Using Pull Requests](https://help.github.com/articles/about-pull-requests/)
- [GitHub Help](https://help.github.com)
