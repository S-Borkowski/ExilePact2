//! The crafting helper's modifier data, embedded from `apps/crafting-helper/data/`:
//! what a parsed item can roll, which tier a modifier on it is, and the rough
//! single-step odds of a currency outcome, or "unknown" where the weights are
//! not established.
//!
//! - [`index`], [`class_data`]: the embedded files, parsed once, lazily.
//! - [`variant_of`]: the item's variant, from its base type; unknown when the
//!   base type is missing or not in the data, never a guess.
//! - [`ClassData::valid_mods`], [`ClassData::mod_tiers`],
//!   [`ClassData::essences_for`], [`ClassData::liquids_for`]: what a variant
//!   and item level can roll, and which essences add a modifier at a floor.
//! - [`join`]: each explicit modifier of a parsed item matched to its data
//!   modifier and tier.
//! - [`add_odds`], [`removal_odds`], [`AFFIX_LIMITS`]: slots, pools and the
//!   chance of one random add or removal.
//! - [`start_weight`], [`trade_offer`], [`trade_query`]: how rare a modifier
//!   is to roll on a base, what a base offers a trade search, and the trade
//!   site's search query for a pick (no address, no league).
//!
//! ```
//! let text = "Item Class: Rings\nRarity: Rare\nDoom Loop\nAmethyst Ring\n--------\nItem Level: 76\n";
//! let item = ch_item::parse(text).unwrap();
//! let options = ch_data::AddOptions {
//!     side: Some(ch_data::Kind::Prefix),
//!     ..Default::default()
//! };
//! let odds = ch_data::add_odds(&item, "prefix-to-maximum-life", 8, &options);
//! assert!((odds.percent().unwrap() - 11.5).abs() < 0.1);
//! ```

mod embedded;
mod join;
mod lookup;
mod model;
mod odds;
mod trade;

pub use embedded::{
    class_data, class_data_by_name, embedded_class, index, trade_stats, EMBEDDED_CLASSES,
    TRADE_STATS_JSON,
};
pub use join::{join, join_one, Join, JoinResult, JoinedMod, Matched, TierRule, Unmatched};
pub use lookup::{variant_of, EssenceFor, ValidMod, ValidMods, VariantUnknown};
pub use model::{
    Base, ClassData, DataMod, Essence, EssenceMatch, Implicit, Index, IndexClass, Kind, Page,
    PageWeights, Section, Tier, WeightBasis,
};
pub use odds::{
    add_odds, affix_limits, item_affix_limits, removal_odds, AddOptions, AffixLimits, LimitClasses,
    LimitRow, Odds, Pool, PoolEntry, Removal, AFFIX_LIMITS, CONTEMPT_EXTRA_PREFIX_LINE,
    CONTEMPT_EXTRA_SUFFIX_LINE, PLACEHOLDER_WEIGHT,
};
pub use trade::{
    searchability, start_weight, trade_offer, trade_query, OfferedMod, Searchability, StartWeight,
    TradeEntry, TradeOffer, TradePick, TradeQuery, TradeSearch, TradeStat, TradeStats,
    WeightSource, DEFAULT_WEIGHT_WITHOUT_PEERS, MAX_TRADE_PICKS,
};
