# Contributing to MoMo

Thanks for taking a look. Bug reports, fixes, and small features are all welcome.

## Reporting a bug

Open an issue with the MoMo version (`momo --version`), your OS and terminal app, what you did, what
you expected, and what happened instead. Logs help a lot: they live in `~/.config/momo/momo-client.log`
and `~/.config/momo/momo-server.log` (or the `sessions/<name>/` folder for a named session).

## Building

You need Rust 1.96 or newer, Zig 0.16, and [`just`](https://github.com/casey/just) with
[`cargo-nextest`](https://nexte.st):

```bash
git clone https://github.com/iiMoham/momo && cd momo
cargo build
./target/debug/momo --version
```

Debug builds keep their config and sessions in `~/.config/momo-dev`, so they never touch an
installed MoMo.

## Making a change

1. Branch from `main`.
2. Keep the change focused, and add or update tests next to the code you touch.
3. Run `just ci && just fork-plugins-test`. CI also runs Linux-only tests, so check its result on
   your pull request before asking for review.
4. Update `README.md` or the page in `docs/` if people will notice the change.
5. Use a lowercase conventional commit subject, such as `fix: keep pane focus after a split`.

`AGENTS.md` has the engineering rules (render purity, performance, the client/server boundary, and
the compatibility names that must not change). Please read it before larger changes.

## License

MoMo is licensed under the Apache License 2.0. By contributing you agree that your contribution is
licensed the same way.
