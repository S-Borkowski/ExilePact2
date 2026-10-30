# Tier relaxation items

What `tests/relax.rs` reads: [`Engine::relax`](../../../src/relax.rs), the
one-target tier relaxation the engine proposes when a target list plans no
route. **Every item text here is constructed**, not copied from the game:
it is written in the measured advanced layout (docs/RUNTIME_DATA_MODELS.md
§ "Item text"), on the Heavy Belt, item level 82, with affix names, tiers
and value ranges taken from `apps/crafting-helper/data/classes/belts.json`
so each modifier joins its data modifier.

The targets are Fire, Cold and Lightning Resistance. On belts the best
resistance essence is the Greater one, which adds tier 3, so a floor of T2
leaves the essence blueprints nothing to bind.

| File | Item | What the tests expect |
|---|---|---|
| `belt-magic-life.txt` | Magic Heavy Belt: maximum Life T2 ("Athlete's"), no suffix. | All three at T2: no route, and the proposal is Fire Resistance from T2 to T3 by Greater Essence of Insulation. With fire at T3: routes whose steps use that essence. |
| `belt-normal.txt` | Normal Heavy Belt, no explicit modifier. | No route at T2 or with fire at T3; with cold at T3 `magic-augment-essence` routes (transmute, then Greater Essence of Thawing), so the proposal is Cold Resistance from T2 to T3. With the three transmutation orbs blocked nothing routes and nothing is proposed. |
| `belt-rare-life-mana.txt` | Rare Heavy Belt: maximum Life T2 and maximum Mana T2 (`ch-app`'s `belt-2p1s.txt` without its suffix). | Routes with all three at T2, so no proposal. |
