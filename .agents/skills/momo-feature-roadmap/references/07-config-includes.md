# 07 config-includes

Source: upstream discussion #4265 "Import/Merge multiple config files" (also resolves #3804).

## Problem

Users keep `config.toml` in dotfiles but need machine-local overrides. There is no way to layer files.

## Behavior

```toml
# ~/.config/momo/config.toml
include = ["config.local.toml", "~/dotfiles/momo/theme.toml"]
```

- `include` is a top-level array of paths. Relative paths resolve against the directory of the file
  that declares them; `~` expands to home.
- Merge order: the including file is the base; each include is deep-merged on top, in order, so
  later files win. Tables merge key-by-key; arrays and scalars replace wholesale (document this:
  `[[keys.command]]` arrays replace, they do not append, unless the plan argues for append).
- Missing include file: skipped with a diagnostic, not an error (so `config.local.toml` can be optional).
  Decide in the plan whether to support an explicit `optional` form instead.
- Nested includes allowed up to depth 4; cycles are detected by canonical path and reported.
- Validation (`momo config check`) reports which file an invalid value came from when practical.
- Settings UI writes (`src/config/write.rs`) keep writing to the main `config.toml` only. If a key
  written by Settings is also set in an include, the include wins after reload; surface a diagnostic
  so the user is not confused.

## Classification

Config loading, shared by server and client (both read config). No wire change.

## Code map (at d11c0c34)

- `src/config/io.rs`: `load()` (~line 131), `config_path()` (~195), `load_live_config()` (~245),
  top-level key list near line 10. Implement merging at the `toml::Value` level before
  deserializing into `Config`, so every section benefits.
- `src/config/write.rs`: `toml_edit` writes; make sure it never inlines include content.
- Client reload: `src/client/config_reload.rs`; server reload: `rg "reload_config" src/app`.
- `HERDR_CONFIG_PATH` override must keep working (includes resolve relative to that file).
- Remote attach: presentation config is client-local; server config is read on the server. Includes
  resolve on the machine that reads the file.

## Tests

- Merge semantics: table deep merge, array replace, scalar override, order.
- Relative + `~` resolution; `HERDR_CONFIG_PATH` base.
- Missing include ⇒ diagnostic, rest loads. Cycle ⇒ diagnostic, no infinite loop. Depth limit.
- Invalid value inside an include falls back like today and names the file.
- Settings write round-trip leaves `include` intact.
- Use temp dirs; no real home directory access.

## Perf

Config load only.

## Stop-and-ask

If the plan wants `include` to be a new table instead of a top-level array, or wants include-aware
Settings writes, ask first.

## Acceptance

`configuration.mdx` "Config file" section documents include; reference JSON row for `include`
(check `config_reference_check.py` handles a top-level array key).
