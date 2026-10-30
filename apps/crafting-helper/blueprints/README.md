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

The seventeen files here are paraphrases, in our own words, of the source
each names in its `source` key; never paste a post's or a guide's text into
one. Each places at most one crafted modifier (an essence, alloy or
liquid emotion) on the item, as 0.5 allows, and the check refuses one
that places two with no removal between them.

Seven of them are state-ordered (`order = "state"`): `close-a-side`,
`essence-desecrate-double-slam`, `magic-augment-essence` and
`rare-clean-then-slam` (since 0.1.1), `weapons-physical-generic` and
`body-armours-single-prefix` (since 0.1.4), and `fractured-essence-swap`
(since 0.2.0). The engine takes the first
step whose guard holds on the item as it stands, so a craft can start
mid-way, and the route preview is the path the craft takes if each step
hits. The first six paraphrase published 0.5 crafting guides (the URL and
its fetch date are in each `source`); `fractured-essence-swap` paraphrases
the owner's statement of 2026-10-30 (an essence that must remove from a
full side never takes its fractured modifier, so with only throwaways
beside it there, one of them goes). Each names in `relies_on` the facts its route leans on
that no official source states. The other ten are listed:
eight from the owner's Discord paste of 2026-10-28, and `fracture-first`
and `full-rare-desecration-rescue` from the guides (0.1.4), which name
their not-established facts in `relies_on` too. Each state-ordered one
places the last missing target on a rare
with no desecrated modifier by a desecration on its side and a reveal
(`desecrate-last`, and `reveal-last` and `light-last` where the blueprint
has no reveal of its own, all bound to `last-remaining`), where the class has a desecrated modifier on that side
at the item's level and neither the bone nor the Necromancy omen is
blocked; otherwise the step taken instead says why. The rules they follow
and the facts table are
[`docs/models/crafting-slots-spec.md`](../../../docs/models/crafting-slots-spec.md);
`crates/ch-engine/tests/slots.rs` holds the engine to the slot model on
the spec's representative crafts for the first four, and
`crates/ch-engine/tests/seeds_v2.rs` walks each of them through a hit and
a miss; `crates/ch-engine/tests/seeds_guides.rs` walks the four from the
guides through a hit, a miss and a start they refuse;
`crates/ch-engine/tests/essence_swap.rs` plans `fractured-essence-swap` on
a ring, a quiver and a Dusk Ring, the survival of an unrevealed
desecrated modifier and each outcome's next step included.
