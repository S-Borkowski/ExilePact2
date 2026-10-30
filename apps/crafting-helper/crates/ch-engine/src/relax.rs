//! Tier relaxation: when a target list plans no route, the one target whose
//! floor, lowered to the tier an essence (or alloy, or liquid emotion)
//! guarantees, lets a route through that currency exist.
//!
//! An essence binds a target only at its floor or better, and the best
//! essence of a modifier is often a tier short of the floor a player asks
//! for (a belt's Greater resistance essences add tier 3). The search tries
//! each such shortfall, one target at a time, and keeps the first that
//! routes. It never relaxes two targets, never raises a floor and proposes
//! nothing when a route already exists.

use ch_blueprint::catalogue::id_from_name;
use ch_data::Section;
use ch_item::Item;
use serde::Serialize;

use crate::context::{Refusal, Target};
use crate::engine::Engine;

/// One target's floor lowered so that a route exists.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Relaxation {
    /// The target's position in the list, from 0.
    pub target: usize,
    pub mod_id: String,
    /// The modifier's text, `#` for each value.
    pub lines: Vec<String>,
    /// The floor in force (the one asked for, or the best reachable tier).
    pub from: u32,
    /// The tier the currency guarantees: the new floor.
    pub to: u32,
    /// The essence, alloy or liquid emotion that guarantees it (catalogue
    /// id).
    pub currency: String,
    /// Its English name.
    pub name: String,
    /// The target list to plan with: the one floor moved, the rest as
    /// given.
    pub targets: Vec<Target>,
    /// The blueprints whose route then takes a step using the currency, in
    /// route order.
    pub routes: Vec<String>,
}

/// A currency that would add a target a few tiers under its floor.
struct Candidate {
    drop: u32,
    target: usize,
    /// Its position among the class's essences then liquid emotions.
    order: usize,
    tier: u32,
    id: String,
    name: String,
}

impl Engine {
    /// The relaxation that lets the targets route, or `None` when they
    /// already route or no single floor lowered to a currency's tier makes
    /// one. Refuses what [`Engine::plan`] refuses.
    ///
    /// Only when the plan has no route: for each target in order, each
    /// data-backed currency of the class (essences, Perfect essences,
    /// alloys, liquid emotions) that adds that target's modifier at a tier
    /// worse than its floor, is not blocked and fits the item level is a
    /// candidate. A candidate holds when re-planning with that floor moved
    /// to the currency's tier gives some route a step whose chosen action
    /// uses the currency. The smallest tier drop wins, then target order,
    /// then data order.
    pub fn relax(
        &self,
        item: &Item,
        targets: &[Target],
        blocked: &[String],
    ) -> Result<Option<Relaxation>, Refusal> {
        let plan = self.plan(item, targets, blocked)?;
        if !plan.routes.is_empty() {
            return Ok(None);
        }
        let Some(data) = ch_data::class_data_by_name(&plan.class) else {
            return Ok(None);
        };

        let mut candidates: Vec<Candidate> = Vec::new();
        for (t, info) in plan.targets.iter().enumerate() {
            if info.section != Section::Normal {
                continue;
            }
            let entries = data.essences.iter().chain(data.liquid.iter());
            for (order, essence) in entries.enumerate() {
                let Some(Some(m)) = essence.matches.get(&plan.variant) else {
                    continue;
                };
                if m.mod_id != info.mod_id || m.tier <= info.floor || essence.ilvl > plan.item_level
                {
                    continue;
                }
                let id = id_from_name(&essence.name);
                // One name may have an entry per modifier; the first will do.
                if blocked.contains(&id) || candidates.iter().any(|c| c.target == t && c.id == id) {
                    continue;
                }
                candidates.push(Candidate {
                    drop: m.tier - info.floor,
                    target: t,
                    order,
                    tier: m.tier,
                    id,
                    name: essence.name.clone(),
                });
            }
        }
        candidates.sort_by_key(|c| (c.drop, c.target, c.order));

        for c in candidates {
            let mut relaxed = targets.to_vec();
            relaxed[c.target].floor = Some(c.tier);
            let Ok(replan) = self.plan(item, &relaxed, blocked) else {
                continue;
            };
            let routes: Vec<String> = replan
                .routes
                .iter()
                .filter(|r| {
                    r.steps
                        .iter()
                        .any(|s| s.action.as_ref().is_some_and(|a| a.uses == c.id))
                })
                .map(|r| r.blueprint.clone())
                .collect();
            if routes.is_empty() {
                continue;
            }
            let info = &plan.targets[c.target];
            return Ok(Some(Relaxation {
                target: c.target,
                mod_id: info.mod_id.clone(),
                lines: info.lines.clone(),
                from: info.floor,
                to: c.tier,
                currency: c.id,
                name: c.name,
                targets: relaxed,
                routes,
            }));
        }
        Ok(None)
    }
}
