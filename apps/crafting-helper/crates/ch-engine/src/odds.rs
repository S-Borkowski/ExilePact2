//! The rough single-step odds of a step's first try on the item: never a
//! simulation, and "unknown" with the reason where the weights are not
//! established (every jewel page, desecration, fluxes). A reveal's are an
//! estimate from the spec's model ("Reveal odds"), its basis naming the
//! not-established facts it rests on.

use ch_blueprint::catalogue::{self, Effect, OmenChange};
use ch_blueprint::{Goal, Rarity as BpRarity};
use ch_data::{Kind, Odds, Removal, Section, WeightBasis, PLACEHOLDER_WEIGHT};

use crate::catalogue::data_entries;
use crate::context::{item_rarity, kind_of, side_word, Ctx, ModInfo, Role, State};
use crate::route::ChosenAction;

/// The data-backed id of Essence of the Abyss, whose modifier (the Mark of
/// the Abyssal Lord) a Fracturing Orb does not pick.
const ABYSS_ESSENCE: &str = "essence-of-the-abyss";

/// The odds basis note of a slam with Omen of Catalysing Exaltation.
pub const CATALYST_NOTE: &str =
    "the catalyst's favour is not counted (Omen of Catalysing Exaltation)";

/// The reveal estimate's switch: exclusive modifiers taken as equally
/// likely (not established: `exclusive-weights-unknown`). Off, an aimed
/// exclusive modifier makes a reveal's chance unknown. The slot model's
/// `EXCLUSIVE_EQUAL_WEIGHT` (tools/crafting_helper/slot_model.py).
pub const EXCLUSIVE_EQUAL_WEIGHT: bool = true;

/// The chance one reveal offers a target, or `None` when it is unknown:
/// the slot model's `reveal_chance` (docs/models/crafting-slots-spec.md,
/// "Reveal odds"), with [`EXCLUSIVE_EQUAL_WEIGHT`].
pub fn reveal_chance(
    exclusive_pool: u32,
    exclusive_hits: f64,
    ordinary: Option<f64>,
    echoes: bool,
) -> Option<f64> {
    reveal_chance_with(
        exclusive_pool,
        exclusive_hits,
        ordinary,
        echoes,
        EXCLUSIVE_EQUAL_WEIGHT,
    )
}

/// [`reveal_chance`] with the equal-weight switch given. `exclusive_pool`
/// is n, the exclusive modifiers that can be offered on the side;
/// `exclusive_hits` h, the aimed ones among them (a tier share may make it
/// fractional); `ordinary` q, one Exalted Orb draw's chance on the side of
/// an aimed ordinary modifier (0 with none aimed, `None` unknown); `echoes`
/// whether Omen of Abyssal Echoes' reroll counts. One set of options
/// misses with `(1 - h/n)(1 - q)^2` (n > 0) or `(1 - q)^3`; the reroll
/// squares the miss. Panics on inputs no item can give.
pub fn reveal_chance_with(
    exclusive_pool: u32,
    exclusive_hits: f64,
    ordinary: Option<f64>,
    echoes: bool,
    equal_weight: bool,
) -> Option<f64> {
    let n = f64::from(exclusive_pool);
    assert!(
        (0.0..=n).contains(&exclusive_hits),
        "exclusive hits {exclusive_hits} of a pool of {exclusive_pool}"
    );
    let q = ordinary?;
    assert!(
        (0.0..=1.0).contains(&q),
        "ordinary chance {q} is not a fraction"
    );
    if exclusive_hits > 0.0 && !equal_weight {
        return None;
    }
    let mut miss = if exclusive_pool > 0 {
        (1.0 - exclusive_hits / n) * (1.0 - q).powi(2)
    } else {
        (1.0 - q).powi(3)
    };
    if echoes {
        miss *= miss;
    }
    Some(1.0 - miss)
}

/// The "rests on" list of a reveal's estimate: its not-established facts.
const REVEAL_ROW: &str = "reveal-exclusive-row";
const REVEAL_DRAWS: &str = "reveal-ordinary-exalt-weights";
const EXCLUSIVE_WEIGHTS: &str = "exclusive-weights-unknown";
const NO_POOL: &str = "necromancy-without-pool";

/// A reveal's estimated odds (docs/models/crafting-slots-spec.md, "Reveal
/// odds"), the same in both orders. The side is the unrevealed modifier's
/// on the item as the step finds it; with none (a listed route evaluates
/// every step on the starting item), the one side every aim is on.
fn reveal_odds(ctx: &Ctx, st: &State, aims: &[usize], action: &ChosenAction) -> Odds {
    if aims.is_empty() {
        return Odds::certain(false, "no remaining target");
    }
    let side = match st.mods.iter().find(|m| m.unrevealed) {
        Some(m) => m.side,
        None => {
            let first = ctx.targets[aims[0]].side;
            if aims.iter().any(|&t| ctx.targets[t].side != first) {
                return Odds::unknown(
                    "the side the reveal offers is not known before the desecration: \
                     the targets are on both sides",
                );
            }
            first
        }
    };
    let word = side_word(side);
    let on_side: Vec<usize> = aims
        .iter()
        .copied()
        .filter(|&t| ctx.targets[t].side == side)
        .collect();
    if on_side.is_empty() {
        return Odds::certain(
            false,
            format!("no target is a {word}, the side the reveal offers"),
        );
    }
    let ordinary: Vec<usize> = on_side
        .iter()
        .copied()
        .filter(|&t| ctx.targets[t].section == Section::Normal)
        .collect();

    // n: the class's exclusive modifiers on the side at the item's level,
    // less those sharing a family with a modifier on the item.
    let on_item: Vec<&String> = st
        .mods
        .iter()
        .filter(|m| !m.unrevealed)
        .flat_map(|m| m.family.iter())
        .collect();
    let valid = ctx.data.valid_mods(ctx.variant, ctx.item_level);
    let pool: Vec<_> = valid
        .desecrated
        .iter()
        .filter(|m| m.kind == side && !m.family.iter().any(|f| on_item.contains(&f)))
        .collect();
    let n = pool.len() as u32;
    // h: the aimed ones, each the share of its reachable tiers at or better
    // than its floor.
    let hits: f64 = on_side
        .iter()
        .filter_map(|&t| {
            let target = &ctx.targets[t];
            let m = pool.iter().find(|m| m.id == target.mod_id)?;
            let counting = m.tiers.iter().filter(|x| x.tier <= target.floor).count();
            Some(counting as f64 / m.tiers.len() as f64)
        })
        .sum();
    // q: one Exalted Orb draw on the side, as an exalt's add odds.
    let q = if ordinary.is_empty() {
        Some(0.0)
    } else {
        match add_chance(ctx, st, &ordinary, &[side], None) {
            Odds::Known { percent, .. } => Some(percent / 100.0),
            Odds::Unknown { reason } => {
                return Odds::unknown(format!(
                    "{reason}, and a reveal's ordinary options are drawn like an Exalted Orb's \
                     (not established: {REVEAL_DRAWS})"
                ))
            }
        }
    };
    let echoes = has_omen_change(action, OmenChange::RerollOptions);
    let Some(chance) = reveal_chance(n, hits, q, echoes) else {
        return Odds::unknown(format!(
            "the weights among the exclusive {word} modifiers are not established \
             ({EXCLUSIVE_WEIGHTS})"
        ));
    };

    let mut rests_on = vec![REVEAL_ROW];
    let mut said: Vec<String> = Vec::new();
    let exalt = |count: &str| {
        match q {
        Some(q) if !ordinary.is_empty() => format!(
            "{count} drawn like an Exalted Orb over the ordinary {word}es, each a target {:.2}% \
             of the time",
            q * 100.0
        ),
        _ => format!("{count} drawn like an Exalted Orb over the ordinary {word}es, none a target"),
    }
    };
    if n > 0 {
        let aimed = if hits > 0.0 {
            rests_on.push(EXCLUSIVE_WEIGHTS);
            format!(", {} of them aimed, each taken as equally likely", trim(hits))
        } else {
            String::new()
        };
        said.push(format!(
            "the first option one of {n} exclusive {word} modifiers{aimed}"
        ));
        said.push(exalt("the other two"));
    } else {
        let any_level = ctx.data.valid_mods(ctx.variant, u32::MAX);
        let why = if !any_level.desecrated.iter().any(|m| m.kind == side) {
            rests_on.push(NO_POOL);
            format!(
                "no exclusive {word} modifier on {}",
                ctx.data.class.as_str()
            )
        } else if !valid.desecrated.iter().any(|m| m.kind == side) {
            format!(
                "item level {} is below every exclusive {word} modifier's level",
                ctx.item_level
            )
        } else {
            format!("every exclusive {word} modifier shares a family with one on the item")
        };
        said.push(format!("{why}: {}", exalt("all three options")));
    }
    if !ordinary.is_empty() {
        rests_on.insert(1, REVEAL_DRAWS);
    }
    if echoes {
        said.push(
            "rerolled once with Omen of Abyssal Echoes when no target is offered".to_string(),
        );
    }
    Odds::estimate(
        chance,
        format!(
            "an estimate: {}; rests on the not-established {}",
            said.join("; "),
            rests_on.join(", ")
        ),
    )
}

/// `0.5`, `1`: a tier share without trailing zeros.
fn trim(x: f64) -> String {
    let s = format!("{x:.2}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// Why a desecration's own odds are unknown: it adds an unrevealed
/// modifier, and the chance that becomes a target is the reveal step's
/// estimate, not the desecration's.
const DESECRATION_UNKNOWN: &str =
    "a desecration only adds an unrevealed modifier: whether it becomes a target is the reveal's chance";

/// The odds of the step's goal on its first try, or `None` for a manual
/// step (no check, no odds).
pub(crate) fn step_odds(
    ctx: &Ctx,
    st: &State,
    goal: Goal,
    aims: &[usize],
    action: &ChosenAction,
) -> Option<Odds> {
    if goal == Goal::Manual || action.effect == Effect::Manual {
        return None;
    }
    match action.effect {
        Effect::Desecrate if goal == Goal::Desecrate => {
            if let Some(odds) = full_side_desecration_odds(st, action) {
                return Some(odds);
            }
            return Some(Odds::unknown(DESECRATION_UNKNOWN));
        }
        Effect::Desecrate => return Some(Odds::unknown(DESECRATION_UNKNOWN)),
        Effect::Reveal => return Some(reveal_odds(ctx, st, aims, action)),
        Effect::Convert => return Some(Odds::unknown("how a flux maps tiers is not established")),
        _ => {}
    }
    Some(match goal {
        Goal::RemoveUnwanted => removal_chance(st, action),
        Goal::RemoveDesecrated => removal_odds(st, action.side.map(kind_of), Removal::Desecrated),
        Goal::Fill => {
            if add_sides(ctx, st, action).is_empty() {
                Odds::certain(false, "no free slot on the allowed side")
            } else {
                Odds::certain(true, "any modifier fills a free slot")
            }
        }
        Goal::AddWanted => add_wanted_odds(ctx, st, aims, action),
        Goal::Desecrate | Goal::Reveal | Goal::Convert => Odds::unknown(format!(
            "{} does not desecrate, reveal or convert: no odds for this goal",
            action.name
        )),
        Goal::Fracture => fracture_odds(ctx, st),
        Goal::Manual => return None,
    })
}

/// A Fracturing Orb's odds: the share of targets among the modifiers it
/// may pick. It never picks a fractured modifier, and (not established) a
/// desecrated one, revealed or not, or the Mark of the Abyssal Lord, the
/// modifier Essence of the Abyss adds (read from the data's lines for it).
fn fracture_odds(ctx: &Ctx, st: &State) -> Odds {
    let mark: Vec<Vec<String>> = data_entries(ctx.data, ABYSS_ESSENCE)
        .iter()
        .map(|e| e.essence.lines.clone())
        .collect();
    let pool: Vec<&ModInfo> = st
        .mods
        .iter()
        .filter(|m| !m.fractured && !m.desecrated && !m.unrevealed && !mark.contains(&m.templates))
        .collect();
    let left_out = "the Mark of the Abyssal Lord and desecrated modifiers left out, \
                    not established: abyss-mark-unfracturable, desecrated-unfracturable";
    if pool.is_empty() {
        return Odds::unknown(format!("no modifier the orb can fracture ({left_out})"));
    }
    let hits = pool
        .iter()
        .filter(|m| matches!(m.role, Role::Wanted(_)))
        .count() as u64;
    let total = pool.len() as u64;
    if hits == total {
        return Odds::certain(
            true,
            format!("every modifier the orb can fracture is a target ({left_out})"),
        );
    }
    Odds::known(
        hits,
        total,
        &format!("modifiers the orb can fracture are targets ({left_out})"),
    )
}

/// A desecration with a Necromancy omen on an item whose omen side is full
/// first removes a modifier, from that side (not established:
/// `full-desecration-same-side`). Its odds are the removal's: the share of
/// non-targets among that side's modifiers a removal may take. The reveal's
/// own chance is not in them. `None` when the side has a free slot.
fn full_side_desecration_odds(st: &State, action: &ChosenAction) -> Option<Odds> {
    let side = action.side.map(kind_of)?;
    if st.free(side) > 0 || st.count(side) == 0 {
        return None;
    }
    let removal = removal_odds(st, Some(side), Removal::Random);
    Some(with_basis_note(
        removal,
        "the desecration's removal on the full side only, the reveal's chance not included \
         (not established: full-desecration-same-side)",
    ))
}

/// How the action's omens choose the removed modifier.
fn removal_rule(action: &ChosenAction) -> Removal {
    let changes: Vec<OmenChange> = action
        .omens
        .iter()
        .filter_map(|id| catalogue::entry(id).and_then(|e| e.omen).map(|o| o.change))
        .collect();
    if changes.contains(&OmenChange::RemoveLowestLevel) {
        Removal::Whittle
    } else if changes.contains(&OmenChange::DesecratedOnly) {
        Removal::Desecrated
    } else {
        Removal::Random
    }
}

pub(crate) fn has_omen_change(action: &ChosenAction, change: OmenChange) -> bool {
    action
        .omens
        .iter()
        .filter_map(|id| catalogue::entry(id).and_then(|e| e.omen))
        .any(|o| o.change == change)
}

/// An essence swap: a Perfect or special essence (`Effect::Swap`), which
/// removes a modifier, like a Chaos Orb, before adding its own. Alloys and
/// liquid emotions swap too but are not essences.
pub(crate) fn is_essence_swap(action: &ChosenAction) -> bool {
    action.data_backed
        && action.effect == Effect::Swap
        && catalogue::data_backed(&action.currency)
            .is_some_and(|d| d.kind == catalogue::Kind::Essence)
}

/// The side an essence swap that adds target `t` removes from: its
/// Crystallisation omen's; with none, `t`'s side when that side is full
/// (not established: `essence-full-side-removal`); otherwise either side
/// (`None`).
pub(crate) fn swap_removal_side(
    ctx: &Ctx,
    st: &State,
    action: &ChosenAction,
    t: usize,
) -> Option<Kind> {
    if let Some(side) = action.side {
        return Some(kind_of(side));
    }
    let side = ctx.targets[t].side;
    (st.free(side) == 0).then_some(side)
}

/// The chance the modifier the action removes is not a wanted one.
pub(crate) fn removal_chance(st: &State, action: &ChosenAction) -> Odds {
    removal_odds(st, action.side.map(kind_of), removal_rule(action))
}

/// `ch-data`'s removal odds on the joined item; on an item the projection
/// changed, the same rules read off its modifiers (a placed target or a
/// filler has no known level).
fn removal_odds(st: &State, side: Option<Kind>, rule: Removal) -> Odds {
    if !st.projected {
        return ch_data::removal_odds(&st.join, &st.wanted_positions(), side, rule);
    }
    let removable: Vec<&ModInfo> = st
        .mods
        .iter()
        .filter(|m| !m.fractured && side.is_none_or(|s| s == m.side))
        .collect();
    let wanted = |m: &ModInfo| matches!(m.role, Role::Wanted(_));
    match rule {
        Removal::Random => {
            if removable.is_empty() {
                return Odds::unknown("no removable modifier");
            }
            let hits = removable.iter().filter(|m| !wanted(m)).count() as u64;
            Odds::known(
                hits,
                removable.len() as u64,
                "removable modifiers are not wanted",
            )
        }
        Removal::Whittle => {
            let Some(levels) = removable
                .iter()
                .map(|m| m.level)
                .collect::<Option<Vec<u32>>>()
            else {
                return Odds::unknown("a modifier's level is not known before the earlier steps");
            };
            let Some(lowest) = levels.iter().min().copied() else {
                return Odds::unknown("no removable modifier");
            };
            let at_lowest: Vec<&&ModInfo> = removable
                .iter()
                .zip(&levels)
                .filter(|(_, l)| **l == lowest)
                .map(|(m, _)| m)
                .collect();
            match at_lowest.as_slice() {
                [only] => Odds::certain(
                    !wanted(only),
                    format!("the lowest modifier level ({lowest}) is unique"),
                ),
                _ => Odds::unknown(format!(
                    "{} modifiers share the lowest modifier level {lowest}",
                    at_lowest.len()
                )),
            }
        }
        Removal::Desecrated => {
            let desecrated: Vec<&&ModInfo> = removable.iter().filter(|m| m.desecrated).collect();
            match desecrated.as_slice() {
                [] => Odds::unknown("no desecrated modifier"),
                [only] => Odds::certain(!wanted(only), "the one desecrated modifier"),
                several => {
                    let hits = several.iter().filter(|m| !wanted(m)).count() as u64;
                    Odds::known(
                        hits,
                        several.len() as u64,
                        "desecrated modifiers are not wanted",
                    )
                }
            }
        }
    }
}

/// The sides one random add of this action can land on: the side omen's,
/// or every side with a free slot under the rarity the item has after the
/// action (a regal makes it Rare). A chaos-style swap also lands where its
/// removal opens a slot.
pub(crate) fn add_sides(ctx: &Ctx, st: &State, action: &ChosenAction) -> Vec<Kind> {
    let after = match action.effect {
        Effect::Upgrade => Some(BpRarity::Rare),
        Effect::Transmute => Some(BpRarity::Magic),
        _ => st.rarity,
    };
    let Some(limits) = after.and_then(|r| ch_data::item_affix_limits(ctx.item, &item_rarity(r)))
    else {
        return Vec::new();
    };
    let swap = action.effect == Effect::Swap;
    let omen_side = action.side.map(kind_of);
    [Kind::Prefix, Kind::Suffix]
        .into_iter()
        .filter(|side| swap || omen_side.is_none_or(|s| s == *side))
        .filter(|side| {
            let free = limits.on(*side) > st.count(*side);
            let opens = swap
                && omen_side.is_none_or(|s| s == *side)
                && st.mods.iter().any(|m| m.side == *side && !m.fractured);
            free || opens
        })
        .collect()
}

fn add_wanted_odds(ctx: &Ctx, st: &State, aims: &[usize], action: &ChosenAction) -> Odds {
    if aims.is_empty() {
        return Odds::certain(false, "no remaining target");
    }
    if action.data_backed {
        let entries = data_entries(ctx.data, &action.uses);
        for &t in aims {
            let target = &ctx.targets[t];
            let tier = entries.iter().find_map(|e| {
                let m = e.essence.matches.get(ctx.variant)?.as_ref()?;
                (m.mod_id == target.mod_id && m.tier <= target.floor).then_some(m.tier)
            });
            if let Some(tier) = tier {
                let add = Odds::certain(
                    true,
                    format!("{} adds {} at T{tier}", action.name, target.mod_id),
                );
                return if is_essence_swap(action) {
                    let side = swap_removal_side(ctx, st, action, t);
                    add.times(&removal_odds(st, side, Removal::Random))
                } else if action.removes {
                    add.times(&removal_chance(st, action))
                } else {
                    add
                };
            }
        }
        return Odds::certain(
            false,
            format!("{} adds no remaining target at its floor", action.name),
        );
    }
    match action.effect {
        Effect::Add | Effect::Augment | Effect::Transmute | Effect::Upgrade | Effect::Swap => {
            let sides = add_sides(ctx, st, action);
            let add = add_chance(ctx, st, aims, &sides, action.min_modifier_level);
            let add = if has_omen_change(action, OmenChange::AddTwo) {
                with_basis_note(add, "for each of the two modifiers added")
            } else {
                add
            };
            // The catalyst's favour has no established size, and the
            // helper cannot read the item's quality: the odds are the
            // plain slam's (docs/models/crafting-slots-spec.md,
            // `catalysing-with-side-omen`).
            let add = if has_omen_change(action, OmenChange::Catalysing) {
                with_basis_note(add, CATALYST_NOTE)
            } else {
                add
            };
            if action.effect == Effect::Swap {
                add.times(&removal_chance(st, action))
            } else {
                add
            }
        }
        Effect::Alchemy => Odds::unknown("an Orb of Alchemy adds four modifiers at once"),
        other => Odds::unknown(format!("{other:?} adds no random modifier")),
    }
}

pub(crate) fn with_basis_note(odds: Odds, note: &str) -> Odds {
    match odds {
        Odds::Known {
            percent,
            hits,
            total,
            basis,
        } => Odds::Known {
            percent,
            hits,
            total,
            basis: format!("{basis}, {note}"),
        },
        unknown => unknown,
    }
}

/// The chance one random add on `sides` rolls any of `aims` at its floor:
/// the aims' counting weight over the pool's. The pool and the weight
/// rules are `ch-data`'s (`ClassData::add_pool`, `Pool::odds`), widened to
/// several targets: certain, weights aside, when the pool's exclusions
/// leave only aims whose every tier counts, and certain not to hit when
/// they leave every aim out and the weights cannot say so.
pub(crate) fn add_chance(
    ctx: &Ctx,
    st: &State,
    aims: &[usize],
    sides: &[Kind],
    min_modifier_level: Option<u32>,
) -> Odds {
    let spawn = matches!(
        ctx.data.page(ctx.variant).map(|p| p.weights.normal),
        Some(WeightBasis::Spawn)
    );
    let no_spawn = || {
        Odds::unknown(format!(
            "{} `{}` has no established spawn weights",
            ctx.data.class, ctx.variant
        ))
    };
    if aims
        .iter()
        .any(|&t| ctx.targets[t].section == Section::Desecrated)
    {
        return Odds::unknown("a desecrated modifier has no established weight");
    }
    if let Some(level) = min_modifier_level {
        if ctx.item_level < level {
            return Odds::unknown(format!(
                "item level {} is below the minimum modifier level {level}",
                ctx.item_level
            ));
        }
    }
    let mut families: Vec<String> = Vec::new();
    // A projected filler takes its slot and excludes no family.
    for m in st.mods.iter().filter(|m| !m.unrevealed && !m.filler) {
        if m.mod_id.is_none() {
            return Odds::unknown(format!(
                "the modifier `{}` did not join the data",
                m.shown()
            ));
        }
        families.extend(m.family.iter().cloned());
    }
    if sides.is_empty() {
        return Odds::unknown("no free slot on the allowed side");
    }
    let pool = ctx.data.add_pool(
        ctx.variant,
        ctx.item_level,
        sides,
        &families,
        min_modifier_level,
    );
    let floor_of = |id: &str| {
        aims.iter()
            .map(|&t| &ctx.targets[t])
            .find(|t| t.mod_id == id)
            .map(|t| t.floor)
    };
    let only_aims = !pool.entries.is_empty()
        && pool
            .entries
            .iter()
            .all(|e| floor_of(e.id).is_some_and(|floor| e.tiers.iter().all(|x| x.tier <= floor)));
    if only_aims {
        return Odds::certain(true, "the pool holds only targets, each tier counting");
    }
    let no_aim = !pool.entries.iter().any(|e| floor_of(e.id).is_some());
    let left_out = || Odds::certain(false, "no target is in the pool");
    if !spawn {
        return if no_aim { left_out() } else { no_spawn() };
    }
    let mut total: u64 = 0;
    for entry in &pool.entries {
        for tier in &entry.tiers {
            match tier.weight {
                Some(w) => total += u64::from(w),
                None if no_aim => return left_out(),
                None => {
                    return Odds::unknown(format!(
                        "{} tier {} has no weight in the data",
                        entry.id, tier.tier
                    ))
                }
            }
        }
    }
    let mut hits: u64 = 0;
    for &t in aims {
        let target = &ctx.targets[t];
        let Some(entry) = pool.entries.iter().find(|e| e.id == target.mod_id) else {
            continue;
        };
        for tier in entry.tiers.iter().filter(|x| x.tier <= target.floor) {
            match tier.weight {
                Some(w) if w != PLACEHOLDER_WEIGHT => hits += u64::from(w),
                _ => {
                    return Odds::unknown(format!(
                        "{} tier {} has no real weight in the data",
                        target.mod_id, tier.tier
                    ))
                }
            }
        }
    }
    if total == 0 {
        return Odds::unknown("the pool is empty");
    }
    Odds::known(hits, total, "weight")
}

/// A target's add odds on its own side, slots aside: what "hardest"
/// compares.
pub(crate) fn target_difficulty(ctx: &Ctx, st: &State, t: usize) -> Option<f64> {
    add_chance(ctx, st, &[t], &[ctx.targets[t].side], None).percent()
}
