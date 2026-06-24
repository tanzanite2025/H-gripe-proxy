# Tauri Plugin Mihomo

> [!IMPORTANT]
>
> This plugin is currently consumed from the local workspace.
> It is not published to `crates.io` or npm.
>
> ```toml
> # Cargo.toml
> tauri-plugin-mihomo = { path = "./crates/tauri-plugin-mihomo" }
> ```
>
> ```json
> {
>   "dependencies": {
>     "tauri-plugin-mihomo-api": "file:./crates/tauri-plugin-mihomo"
>   }
> }
> ```

`tauri-plugin-mihomo` is the Tauri-side bridge used by this repository to talk to Mihomo over HTTP and socket transports. Its Tauri invoke surface is intentionally retired; app code should enter Mihomo through Rust-owned commands, `core::runtime_snapshot`, or `core::runtime_bridge`, while the npm package exports generated TypeScript bindings only.

## Workspace Usage

In this repository:

- Rust code uses the plugin through the workspace dependency defined in `Cargo.toml`
- Frontend code uses `tauri-plugin-mihomo-api` only as a generated type source via the local `file:` dependency in `package.json`
- No external upstream repository path is required for normal development

## Testing

[`nextest`](https://github.com/nextest-rs/nextest) is recommended for plugin test runs.

By default, tests use the Mihomo socket endpoint. You can switch to HTTP mode by setting `MIHOMO_SOCKET=0`.

```shell
# Excludes restart/reload_config because they reload Mihomo state
cargo nextest run mihomo_

# Test reload_config separately
cargo nextest run reload

# Test restart separately
cargo nextest run restart
```

## Frontend Build

```shell
pnpm install
pnpm build
```

## Regenerate Bindings

If you modify `model.rs`, regenerate exported frontend bindings with:

```shell
cargo test export_bindings
```
