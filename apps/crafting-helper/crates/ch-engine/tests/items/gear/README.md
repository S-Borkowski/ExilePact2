# Equipment seed walks

The item states `tests/seeds_gear.rs` walks the five equipment seed
blueprints over (`apps/crafting-helper/blueprints/`), one folder per
blueprint id. **Every file here is constructed**, not copied from the game:
it is written in the measured advanced layout (docs/RUNTIME_DATA_MODELS.md
§ "Item text"), with affix names, tiers and value ranges taken from
`apps/crafting-helper/data/` so each modifier joins its data modifier.
How a flux maps tiers and what a reveal offers are not established; the
flux and reveal results below are picked to exercise the guide, not
measured.

Real fixtures used beside these, from `crates/ch-item/fixtures/real/`:

- `gloves-rare-rune-desecrated-ctrl-alt-c.txt`, the owner's Sirenscale
  Gloves: a reveal in `gloves-crit-melee`'s miss path. The `gloves-crit-melee`
  files copy its layout (quality, rune socket, item level 81), and files 16
  and 17 are it with its desecrated "of Exile" still unrevealed, then gone.
- `rings-normal-implicit-advanced.txt`, the Amethyst Ring: the ring files
  copy its base, item level 76 and chaos resistance implicit. It is Normal,
  so no ring walk starts on it.

A file used twice in a walk is the same item reached again (a miss loop
that lands where it started).

## fill-desecrate-exalt (Amethyst Ring, item level 76)

Targets: maximum Life T3, fire and cold resistance T4, chaos resistance T3.

| File | State |
|---|---|
| `01-magic.txt` | Magic, maximum Mana T5: an essence can make it Rare. |
| `02-essence-life.txt` | Rare after Essence of the Body: Mana T5, Life T3. |
| `03-fill-fire.txt` | Plus fire resistance T4 (exalt). Also the miss path's start: two targets on a Rare. |
| `04-desecrated.txt` | Plus an unrevealed suffix. |
| `05-revealed-chaos.txt` | Revealed: desecrated chaos resistance T3. |
| `06-slam-cold.txt` | Plus cold resistance T3 (the last slam). |
| `07-fill-miss.txt` | `03` plus Accuracy T3: an exalt that misses. |
| `08-whittled.txt` | Accuracy whittled away, Strength T5 in its place. |
| `09-fill-cold.txt` | Plus cold resistance T3: the suffixes are full. |
| `10-desecrated.txt` | Strength gone, an unrevealed suffix in its place. |
| `11-revealed-wrong.txt` | Revealed as desecrated cold and chaos resistance (not a target). |
| `12-light.txt` | The desecrated one gone (Omen of Light). |
| `13-revealed-chaos.txt` | Revealed again: desecrated chaos resistance T3. |

## rings-resist-whittle (Amethyst Ring, item level 76)

Targets: fire, cold and lightning resistance, each T3.

| File | State |
|---|---|
| `01-start.txt` | Rare: maximum Mana T5, Strength T5. |
| `02-chaos-fire.txt` | Strength chaosed into fire resistance T3. |
| `03-whittle-cold.txt` | Mana whittled into cold resistance T3. |
| `04-desecrated.txt` | Plus an unrevealed suffix. |
| `05-revealed-lightning.txt` | Revealed: desecrated lightning resistance T3. |
| `06-chaos-below-floor.txt` | Strength chaosed into fire resistance T5, below the floor. |
| `07-revealed-wrong.txt` | `04` revealed as desecrated cold and chaos resistance. |
| `08-whittle-lightning.txt` | That one whittled into lightning resistance T3. |

## rings-flat-attack (Amethyst Ring, item level 76)

Targets: physical, lightning and cold damage to attacks, each T5; fire
resistance T4.

| File | State |
|---|---|
| `01-start.txt` | Rare: physical damage to attacks T4, Strength T5, Mana Regeneration T4. |
| `02-breach.txt` | Strength swapped for Essence of the Breach's quality-cap prefix (it joins no data modifier). |
| `03-catalysts.txt` | The same at 40% quality. |
| `04-chaos-lightning.txt` | The quality-cap prefix chaosed into lightning damage to attacks T4. |
| `05-slam-cold.txt` | Plus cold damage to attacks T4. |
| `06-whittle-fire.txt` | Mana Regeneration whittled into fire resistance T4. |
| `07-chaos-miss.txt` | `03` with the quality-cap prefix chaosed into Accuracy T3. |
| `08-slam-miss.txt` | `04` plus maximum Mana T5: a slam that misses. |

## gloves-crit-melee (Sirenscale Gloves, item level 81)

Hit path targets: critical damage bonus T2, lightning damage to attacks T3,
attack speed T2. Miss path targets: increased Energy Shield T2, attack
speed T2.

| File | State |
|---|---|
| `01-start.txt` | Rare: maximum Energy Shield T1; lightning resistance T3, cold resistance T5. |
| `02-exalt-suffix.txt` | Plus Intelligence T5. |
| `03-hysteria.txt` | Intelligence swapped for critical damage bonus T2 (Essence of Hysteria). |
| `04-block-prefix.txt` | Plus an unrevealed prefix. |
| `05-blocker-revealed.txt` | Revealed as the desecrated energy shield and life hybrid T4. |
| `06-slam-lightning.txt` | Plus lightning damage to attacks T3. |
| `07-light.txt` | The desecrated blocker gone. |
| `08-slam-again.txt` | Plus maximum Mana T1. |
| `09-desecrate-suffix.txt` | Cold resistance gone, an unrevealed suffix in its place. |
| `10-reveal-attack-speed.txt` | Revealed: desecrated attack speed T1. |
| `11-miss-block-prefix.txt` | `02` plus an unrevealed prefix. |
| `12-miss-blocker-revealed.txt` | Revealed as the desecrated hybrid T4. |
| `13-miss-slam-mana.txt` | Plus maximum Mana T1: the slam misses. |
| `14-miss-light.txt` | The desecrated blocker gone. |
| `15-miss-slam-again.txt` | Plus increased Energy Shield T2. |
| `16-miss-desecrate-suffix.txt` | Intelligence gone, an unrevealed suffix: the real gloves before their reveal. |
| `17-miss-light-again.txt` | The real gloves with "of Exile" gone (Omen of Light). |
| `18-miss-reveal-attack-speed.txt` | `16` revealed as desecrated attack speed T1. |

## amulets-resist-spirit (Jade Amulet, item level 82)

Targets: chaos and fire resistance T2, maximum Life T2, maximum Energy
Shield T3, Spirit T2.

| File | State |
|---|---|
| `01-start.txt` | Rare: maximum Mana T6, Mana Regeneration T4. |
| `02-annulled.txt` | Mana annulled. |
| `03-chaos-cold.txt` | Mana Regeneration chaosed into cold resistance T2. |
| `04-void-chaos.txt` | Void Flux: chaos resistance T2. |
| `05-slam-lightning.txt` | Plus lightning resistance T2 (a suffix slam). |
| `06-blazing-fire.txt` | Blazing Flux: fire resistance T2. |
| `07-desecrate-block.txt` | Plus an unrevealed suffix. |
| `08-blocker-revealed.txt` | Revealed as desecrated Intelligence T1, the blocker. |
| `09-slam-life.txt` | Plus maximum Life T2. |
| `10-slam-es.txt` | Plus maximum Energy Shield T3. |
| `11-light.txt` | The blocker gone. |
| `12-desecrate-spirit.txt` | Plus an unrevealed prefix. |
| `13-spirit.txt` | Revealed: desecrated Spirit T1. |
| `14-void-below-floor.txt` | `03` after a Void Flux that landed chaos resistance T5. |
| `15-slam-miss.txt` | `08` plus maximum Mana T3: the slam misses. |
| `16-spirit-wrong.txt` | `12` revealed as the desecrated global defences prefix. |
