# Blocker advice items

What `tests/blocker.rs` reads: [`Engine::blocker_advice`](../../../src/blocker.rs),
the desecrated modifiers a reveal step names to take when the target is not
offered. **The item text here is constructed**, not copied from the game: it
is written in the measured advanced layout (docs/RUNTIME_DATA_MODELS.md §
"Item text"), on the Heavy Belt, item level 82, with the affix name, tier
and value range taken from `apps/crafting-helper/data/classes/belts.json`
so the modifier joins its data modifier. It is `tests/items/relax/`'s magic
belt, the relaxation's example.

The targets are the relaxed list: Fire Resistance T3 (Greater Essence of
Insulation's tier), Cold and Lightning Resistance T2.

| File | Item | What the tests expect |
|---|---|---|
| `belt-magic-life.txt` | Magic Heavy Belt: maximum Life T2 ("Athlete's"), no suffix. | `essence-desecrate-double-slam`: the essence, a suffix desecration, its reveal, a suffix exalt; the reveal names the two desecrated suffixes that share the `Strength` family with `suffix-to-strength`, after above before. `magic-augment-essence`: its desecration is its last add, so no advice. |
