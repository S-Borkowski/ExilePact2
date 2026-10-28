//! Which app is updated ([`Product`]) and a release's number ([`Version`]).

use std::fmt;

/// The app an [`crate::Updater`] works for, as data: the name its texts
/// use, the prefix of its release tags and the file name of its exe asset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Product {
    /// The name the Settings window's texts use ("Quest Tracker").
    pub name: &'static str,
    /// The prefix of the product's release tags; the tag is this and exactly
    /// `<major>.<minor>.<patch>` (`tools/<app>/release.py`'s `TAG_PREFIX`).
    pub tag_prefix: &'static str,
    /// The file name of the exe asset a release carries. Its signature is
    /// this and [`SIGNATURE_SUFFIX`].
    pub exe_name: &'static str,
}

/// The suffix of an exe's signature asset: minisign's signature text.
pub const SIGNATURE_SUFFIX: &str = ".minisig";

/// The quest tracker, as its releases name it.
pub const QUEST_TRACKER: Product = Product {
    name: "Quest Tracker",
    tag_prefix: "quest-tracker-v",
    exe_name: "quest-tracker.exe",
};

/// The waystone roller, as its releases name it.
pub const WAYSTONE_ROLLER: Product = Product {
    name: "Waystone Roller",
    tag_prefix: "waystone-roller-v",
    exe_name: "waystone-roller.exe",
};

/// The crafting helper, as its releases name it.
pub const CRAFTING_HELPER: Product = Product {
    name: "Crafting Helper",
    tag_prefix: "crafting-helper-v",
    exe_name: "crafting-helper.exe",
};

impl Product {
    /// The file name of the exe's signature asset, `<exe>.minisig`.
    pub fn signature_name(&self) -> String {
        format!("{}{SIGNATURE_SUFFIX}", self.exe_name)
    }

    /// The release tag of `version`, `<prefix><major>.<minor>.<patch>`.
    pub fn tag(&self, version: Version) -> String {
        format!("{}{version}", self.tag_prefix)
    }

    /// The version a tag of this product names: the prefix, then
    /// [`Version::parse`]; `None` for another product's tag or a bad one.
    pub fn version_of_tag(&self, tag: &str) -> Option<Version> {
        Version::parse(tag.strip_prefix(self.tag_prefix)?)
    }
}

/// A release's `<major>.<minor>.<patch>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

impl Version {
    pub const fn new(major: u64, minor: u64, patch: u64) -> Version {
        Version {
            major,
            minor,
            patch,
        }
    }

    /// Exactly three decimal numbers joined by dots, nothing before or after
    /// (no sign, no space, no `-rc1`); `None` for anything else.
    pub fn parse(text: &str) -> Option<Version> {
        let mut parts = text.split('.');
        let mut next = || -> Option<u64> {
            let p = parts.next()?;
            if p.is_empty() || !p.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            p.parse().ok()
        };
        let v = Version::new(next()?, next()?, next()?);
        parts.next().is_none().then_some(v)
    }

    /// A package version as Cargo writes it (`env!("CARGO_PKG_VERSION")` of
    /// the app's own crate), read up to any `-` or `+` suffix; 0.0.0 should
    /// it ever not parse. This crate cannot know the app's version, so the
    /// app passes its own.
    pub fn of_package(package_version: &str) -> Version {
        let core = package_version
            .split(['-', '+'])
            .next()
            .unwrap_or(package_version);
        Version::parse(core).unwrap_or(Version::new(0, 0, 0))
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}
