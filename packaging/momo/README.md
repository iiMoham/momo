# MoMo packaging

- `install.sh` – the one-line installer published with every release
  (`curl -fsSL https://github.com/iiMoham/momo/releases/latest/download/install.sh | sh`).
  It downloads `momo-<os>-<arch>`, checks it against `SHA256SUMS`, and installs to
  `~/.local/bin/momo` (`MOMO_INSTALL_DIR`, `MOMO_VERSION` override).
- `release_manifest.py` – writes `latest.json` (read by `momo update` and remote SSH installs) and
  `SHA256SUMS` from the built binaries. `latest.json` keeps up to 20 earlier releases so remote hosts
  can always get the exact build a client runs.
- `tests/` – run with `just fork-plugins-test`.

## Cutting a release

Releases are built by `.github/workflows/momo-release.yml` when a `momo-v<base>-momo.<N>` tag is
pushed. `<base>` must equal the `Cargo.toml` version (the upstream momo version MoMo is based on);
`<N>` counts MoMo releases on that base and restarts at 1 after an upstream version bump. The
annotated tag message becomes the release notes shown by `momo update`.

```bash
git switch main && git pull
just ci && just fork-plugins-test
# Write the notes to a file; --cleanup=verbatim keeps "### Added" headings,
# which git would otherwise strip as comments.
git tag -a momo-v0.9.1-momo.2 --cleanup=verbatim -F notes.md
git push origin momo-v0.9.1-momo.2
```

The workflow builds macOS (arm64, x86_64) and Linux (x86_64, arm64, static musl) binaries, checks
the version identity, and publishes the GitHub Release with the binaries, `SHA256SUMS`,
`latest.json`, and `install.sh`.
