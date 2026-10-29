# Blueprints

The crafting helper's crafting techniques, one TOML file each, named
`<id>.toml`. Every `.toml` file here is embedded in the app at build time
(`ch-blueprint`'s `build.rs`); this README is not.

- How to write one, every key, the ids and the outcome rules:
  [`docs/crafting-helper/blueprints.md`](../../../docs/crafting-helper/blueprints.md)
- Check a file or this whole folder after an edit:

  ```sh
  cargo run --manifest-path apps/crafting-helper/Cargo.toml --locked -p ch-engine --example check_blueprints -- apps/crafting-helper/blueprints
  ```

  Each error is one `<path>:<line>:<col>: <message>` line. Then run
  `cargo test --manifest-path apps/crafting-helper/Cargo.toml --locked -p ch-engine --test shipped`.

The ten files here are paraphrases, in our own words, of the source each
names in its `source` key; never paste a post's text into one.
