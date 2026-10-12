// Public package surface: only the Gmail OAuth adapter and contract modules.
// The `gmail-oauth-worker*` modules are intentionally not re-exported: they own
// the privileged worker process boundary, not the library API. They are bundled
// straight from source by `scripts/build-sidecar.mjs` into the Tauri
// `externalBin` `cancan-gmail-connector` that the Rust host spawns, and tests
// reach them by direct relative import. Keep this export list unchanged when
// adding or renaming worker modules.
export * from "./gmail-oauth-adapter";
export * from "./gmail-oauth";
