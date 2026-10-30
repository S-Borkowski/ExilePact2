"""docs/crafting-helper/crafting-mechanics.md: the crafting knowledge base keeps its shape.

The doc is written by hand, in our own words, from poe2db.tw pages and the
owner's Discord paste. Its readers (the M4 blueprints, and whoever checks a
claim against the game) depend on a fixed shape, which these cases pin:

- every top-level section exists, in order (`test_required_headings`);
- every currency, omen and essence a blueprint can name is described
  somewhere (`test_required_terms`);
- every entry in the mechanics sections says where its claim came from, a
  poe2db URL with the fetch date, or "not established" and then a line under
  `## Gaps` (`test_every_entry_has_a_source`);
- the fourteen chunks of the owner's paste are each one `### Post <n>`
  (`test_posts_one_to_fourteen`);
- `## Gaps` says the wiki was not read (`test_gaps_name_the_wiki`);
- corruption, Vaal Orb included, is out of scope rather than described;
- every fact taken from the crafting guides of 2026-10-30 carries a label
  (community guide, documented or not established) and its source, and
  `## Sources` names each guide with that date (`test_guide_facts_are_labelled`).

Each check is a small function over the doc's text, and each keeps a negative
control beside it: a synthetic doc the function must refuse, so a check that
can no longer fail is caught here rather than trusted.
"""
import re
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

DOC = ROOT / "docs" / "crafting-helper" / "crafting-mechanics.md"

REQUIRED_HEADINGS = [
    "Sources",
    "Item basics",
    "Currency",
    "Greater and Perfect currency",
    "Essences",
    "Desecration",
    "Omens",
    "Alloys",
    "Fluxes",
    "Catalysts",
    "Liquid emotions",
    "Out of scope",
    "Crafting patterns seen",
    "Gaps",
]

# The sections whose every `###` entry must carry a `Source:` line.
SOURCED_SECTIONS = REQUIRED_HEADINGS[
    REQUIRED_HEADINGS.index("Currency"):REQUIRED_HEADINGS.index("Liquid emotions") + 1
]

REQUIRED_TERMS = [
    "Orb of Transmutation",
    "Orb of Augmentation",
    "Regal Orb",
    "Exalted Orb",
    "Chaos Orb",
    "Orb of Annulment",
    "Divine Orb",
    "Orb of Alchemy",
    "Fracturing Orb",
    "Orb of Chance",
    "Vaal Orb",
    "Greater Exalted Orb",
    "Perfect Exalted Orb",
    "Greater Chaos Orb",
    "Perfect Chaos Orb",
    "Omen of Whittling",
    "Omen of Sinistral Exaltation",
    "Omen of Dextral Exaltation",
    "Omen of Greater Exaltation",
    "Omen of Sinistral Annulment",
    "Omen of Dextral Annulment",
    "Omen of Sinistral Crystallisation",
    "Omen of Dextral Crystallisation",
    "Omen of Sinistral Erasure",
    "Omen of Dextral Erasure",
    "Omen of Light",
    "Omen of Abyssal Echoes",
    "Omen of the Sovereign",
    "Omen of the Liege",
    "Omen of the Blackblooded",
    "Gnawed Jawbone",
    "Essence of the Abyss",
    "Essence of Hysteria",
    "Lesser Essence",
    "Greater Essence",
    "Perfect Essence",
    "Void Flux",
    "Minimum Modifier Level",
]

POST_COUNT = 14
POST_SOURCE = "Source: Discord post {n}, owner's paste, 2026-10-28"

SOURCE_LINE = re.compile(
    r"^Source: .*https://poe2db\.tw/us/\S+.*\b\d{4}-\d{2}-\d{2}\b", re.MULTILINE
)
NOT_ESTABLISHED = re.compile(r"^Source: not established\b", re.MULTILINE)

# The crafting guides read for issue #35: the owner's paste of the Mobalytics
# pages, and the maxroll page the issue records as reviewed.
GUIDE_DATE = "2026-10-30"
MOBALYTICS = "https://mobalytics.gg/poe-2/guides/"
MAXROLL_HOW_TO = "https://maxroll.gg/poe2/resources/how-to-craft-in-path-of-exile-2"
PATCH_NOTES_050 = "https://www.pathofexile.com/forum/view-thread/3932540"
GUIDE_URLS = [MOBALYTICS + slug for slug in (
    "ssf-crafting",
    "lightning-arrow-crafting",
    "isolation-crafting",
    "fracturing-orbs",
    "omen-of-recombination",
    "campaign-martial-weapon-crafting",
    "crafting-basics-part-1",
    "essences",
    "0-3-crafting-changes",
)] + [MAXROLL_HOW_TO]

# A guide fact is tagged in place: "(label: <label>; <source URL>, ... 2026-10-30)".
LABEL_TAG = re.compile(
    r"\(label: (community guide|documented|not established); ([^()]*?\b"
    + GUIDE_DATE + r"\b[^()]*)\)"
)
GUIDE_SECTION = "Crafting principles from the guides"

# Where the guide facts sit, and a URL one of the place's tags must name.
# ("##", "###") is a `###` entry; ("##", "- **name") a bold-named bullet; ("##",
# None) the section's own text before its first `###`.
LABELLED_PLACES = [
    (("Item basics", "- **Crafted and desecrated modifiers"), MOBALYTICS),
    (("Item basics", "- **Hybrid modifiers"), MOBALYTICS),
    (("Currency", "Orb of Alchemy"), MOBALYTICS),
    (("Currency", "Fracturing Orb"), MOBALYTICS),
    (("Greater and Perfect currency", "Greater Orb of Transmutation"), MOBALYTICS),
    (("Greater and Perfect currency", "Greater Orb of Augmentation"), MOBALYTICS),
    (("Greater and Perfect currency", "Perfect Exalted Orb"), MOBALYTICS),
    (("Essences", "Perfect Essence"), MOBALYTICS),
    (("Essences", "Essence of the Abyss"), MOBALYTICS),
    (("Desecration", "Desecration and reveal"), MAXROLL_HOW_TO),
    (("Omens", "Omen of Recombination"), MOBALYTICS),
    (("Alloys", None), PATCH_NOTES_050),
    (("Out of scope", "- **The reforge bench"), MOBALYTICS),
    ((GUIDE_SECTION, "- **Isolation"), MOBALYTICS),
    ((GUIDE_SECTION, "- **Gamble last"), MOBALYTICS),
    ((GUIDE_SECTION, "- **Restart a weak magic start"), MOBALYTICS),
    ((GUIDE_SECTION, "- **Item-level breakpoints"), MOBALYTICS),
    ((GUIDE_SECTION, "- **Alchemy and the reforge fallback"), MOBALYTICS),
]


def split_sections(text, level):
    """Return [(title, body)] for each heading of exactly `level` hashes.

    The text before the first such heading is dropped; a deeper heading stays
    inside its parent's body.
    """
    marker = re.compile(r"^" + "#" * level + r" (.+?)\s*$", re.MULTILINE)
    matches = list(marker.finditer(text))
    out = []
    for i, m in enumerate(matches):
        end = matches[i + 1].start() if i + 1 < len(matches) else len(text)
        out.append((m.group(1), text[m.end():end]))
    return out


def section(text, title):
    for name, body in split_sections(text, 2):
        if name == title:
            return body
    return None


def heading_problems(text):
    titles = [name for name, _ in split_sections(text, 2)]
    problems = [f"missing ## {h}" for h in REQUIRED_HEADINGS if h not in titles]
    present = [t for t in titles if t in REQUIRED_HEADINGS]
    if not problems and present != REQUIRED_HEADINGS:
        problems.append(f"out of order: {present}")
    return problems


def missing_terms(text):
    flat = " ".join(text.split())
    return [term for term in REQUIRED_TERMS if term not in flat]


def source_problems(text):
    """Every `###` entry of a sourced section names a poe2db page and a date,
    or says "not established" and is then named under `## Gaps`."""
    gaps = " ".join((section(text, "Gaps") or "").split())
    problems = []
    for title in SOURCED_SECTIONS:
        body = section(text, title)
        if body is None:
            problems.append(f"## {title}: missing")
            continue
        entries = split_sections(body, 3)
        if not entries:
            problems.append(f"## {title}: no ### entries")
        for name, entry in entries:
            if SOURCE_LINE.search(entry):
                continue
            if NOT_ESTABLISHED.search(entry):
                if name not in gaps:
                    problems.append(f"### {name}: not established but absent from ## Gaps")
                continue
            problems.append(f"### {name}: no Source: line with a poe2db URL and date")
    return problems


def post_problems(text):
    body = section(text, "Crafting patterns seen")
    if body is None:
        return ["## Crafting patterns seen: missing"]
    entries = split_sections(body, 3)
    names = [name for name, _ in entries]
    expected = [f"Post {n}" for n in range(1, POST_COUNT + 1)]
    problems = []
    if names != expected:
        problems.append(f"entries are {names}, expected {expected}")
    for n, (name, entry) in enumerate(entries, start=1):
        if POST_SOURCE.format(n=n) not in entry:
            problems.append(f"### {name}: no '{POST_SOURCE.format(n=n)}' line")
    return problems


def gap_problems(text):
    body = section(text, "Gaps")
    if body is None:
        return ["## Gaps: missing"]
    flat = " ".join(body.split()).lower()
    problems = []
    if "poe2wiki" not in flat:
        problems.append("## Gaps does not name poe2wiki")
    if "not read" not in flat:
        problems.append("## Gaps does not say poe2wiki was not read")
    return problems


def out_of_scope_problems(text):
    body = section(text, "Out of scope")
    if body is None:
        return ["## Out of scope: missing"]
    problems = []
    flat = " ".join(body.split())
    if "Vaal Orb" not in flat:
        problems.append("## Out of scope does not name Vaal Orb")
    if "corrupt" not in flat.lower():
        problems.append("## Out of scope does not name corruption")
    for title in SOURCED_SECTIONS:
        for name, _ in split_sections(section(text, title) or "", 3):
            if name == "Vaal Orb":
                problems.append(f"Vaal Orb has an entry under ## {title}")
    return problems


def bullets(body):
    """Return each top-level `- ` bullet of `body`, its indented lines joined."""
    out = []
    inside = False
    for line in body.splitlines():
        if line.startswith("- "):
            out.append(line)
            inside = True
        elif inside and line.startswith("  ") and line.strip():
            out[-1] += " " + line.strip()
        else:
            inside = False
    return out


def place_text(text, title, key):
    """The text of one labelled place: a `###` entry, a bold-named bullet, or
    (key None) a section's text before its first `###`. None when absent."""
    body = section(text, title)
    if body is None:
        return None
    if key is None:
        return re.split(r"^### ", body, maxsplit=1, flags=re.MULTILINE)[0]
    if key.startswith("- "):
        found = [b for b in bullets(body) if b.startswith(key)]
        return found[0] if found else None
    return dict(split_sections(body, 3)).get(key)


def guide_label_problems(text):
    """Each guide fact's place carries a label tag dated 2026-10-30 that names
    its source; `## Sources` lists each guide with that date; the principles
    section sits after the crafting patterns and before `## Gaps`; and the
    alloy question has left `## Gaps`."""
    problems = []
    for (title, key), url in LABELLED_PLACES:
        where = f"## {title}" + (f" / {key}" if key else "")
        body = place_text(text, title, key)
        if body is None:
            problems.append(f"{where}: missing")
            continue
        tags = LABEL_TAG.findall(" ".join(body.split()))
        if not tags:
            problems.append(f"{where}: no '(label: ...; ..., {GUIDE_DATE})' tag")
        elif not any(url in source for _, source in tags):
            problems.append(f"{where}: no label tag names {url}")
    source_bullets = bullets(section(text, "Sources") or "")
    for url in GUIDE_URLS:
        named = re.compile(re.escape(url) + r"(?![\w/-])")
        listed = [b for b in source_bullets if named.search(b)]
        if not listed:
            problems.append(f"## Sources does not list {url}")
        elif not any(GUIDE_DATE in b for b in listed):
            problems.append(f"## Sources lists {url} without {GUIDE_DATE}")
    titles = [name for name, _ in split_sections(text, 2)]
    if GUIDE_SECTION not in titles:
        problems.append(f"## {GUIDE_SECTION}: missing")
    elif not (titles.index("Crafting patterns seen") < titles.index(GUIDE_SECTION)
              < titles.index("Gaps")):
        problems.append(f"## {GUIDE_SECTION} is not between the patterns and ## Gaps")
    if "Alloys and the crafted limit" in (section(text, "Gaps") or ""):
        problems.append("## Gaps still holds the alloy question the 0.5.0 notes settle")
    return problems


def minimal_doc():
    """A synthetic doc with the right shape, for the negative controls."""
    parts = ["# Title\n"]
    for title in REQUIRED_HEADINGS:
        parts.append(f"## {title}\n")
        if title in SOURCED_SECTIONS:
            parts.append(f"### {title} entry\nText.\n"
                         f"Source: https://poe2db.tw/us/Omen, fetched 2026-10-28\n")
        elif title == "Crafting patterns seen":
            for n in range(1, POST_COUNT + 1):
                parts.append(f"### Post {n}\n{POST_SOURCE.format(n=n)}\n")
        elif title == "Gaps":
            parts.append("poe2wiki was not read.\n")
        elif title == "Out of scope":
            parts.append("Vaal Orb and every corruption outcome.\n")
    parts.append(" ".join(REQUIRED_TERMS) + "\n")
    return "\n".join(parts)


class MechanicsDoc(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.text = DOC.read_text(encoding="utf-8")

    def test_required_headings(self):
        self.assertEqual(heading_problems(self.text), [])
        # Negative controls: a dropped heading and a swapped pair are refused.
        good = minimal_doc()
        self.assertEqual(heading_problems(good), [])
        self.assertTrue(heading_problems(good.replace("## Fluxes\n", "## Flux\n")))
        swapped = good.replace("## Alloys\n", "## @@\n").replace(
            "## Fluxes\n", "## Alloys\n").replace("## @@\n", "## Fluxes\n")
        self.assertTrue(heading_problems(swapped))

    def test_required_terms(self):
        self.assertEqual(missing_terms(self.text), [])
        good = minimal_doc()
        self.assertEqual(missing_terms(good), [])
        self.assertEqual(missing_terms(good.replace("Void Flux", "Void")), ["Void Flux"])

    def test_every_entry_has_a_source(self):
        self.assertEqual(source_problems(self.text), [])
        good = minimal_doc()
        self.assertEqual(source_problems(good), [])
        # An entry with no source line, or a source with no date, is refused.
        no_source = good.replace(
            "Source: https://poe2db.tw/us/Omen, fetched 2026-10-28\n", "", 1)
        self.assertTrue(source_problems(no_source))
        no_date = good.replace("fetched 2026-10-28", "fetched recently", 1)
        self.assertTrue(source_problems(no_date))
        # "not established" passes only when ## Gaps names the entry.
        unsourced = good.replace(
            "Source: https://poe2db.tw/us/Omen, fetched 2026-10-28",
            "Source: not established", 1)
        self.assertTrue(source_problems(unsourced))
        self.assertEqual(source_problems(unsourced.replace(
            "poe2wiki was not read.", "poe2wiki was not read. Currency entry: open.")), [])

    def test_posts_one_to_fourteen(self):
        self.assertEqual(post_problems(self.text), [])
        good = minimal_doc()
        self.assertEqual(post_problems(good), [])
        self.assertTrue(post_problems(good.replace("### Post 14\n", "")))
        self.assertTrue(post_problems(good.replace("Discord post 3,", "Discord post 4,")))
        # Post 7 is the paste's pointer to video guides, recorded as no craft.
        post7 = dict(split_sections(section(self.text, "Crafting patterns seen"), 3))["Post 7"]
        self.assertIn("no craft", post7.lower())

    def test_gaps_name_the_wiki(self):
        self.assertEqual(gap_problems(self.text), [])
        good = minimal_doc()
        self.assertEqual(gap_problems(good), [])
        self.assertTrue(gap_problems(good.replace("poe2wiki was not read.", "Nothing.")))

    def test_vaal_orb_and_corruption_are_out_of_scope(self):
        self.assertEqual(out_of_scope_problems(self.text), [])
        good = minimal_doc()
        self.assertEqual(out_of_scope_problems(good), [])
        moved = good.replace("### Currency entry\n", "### Vaal Orb\n")
        self.assertTrue(out_of_scope_problems(moved))

    def test_guide_facts_are_labelled(self):
        self.assertEqual(guide_label_problems(self.text), [])
        # Negative controls, each a one-place edit of the real doc. A synthetic
        # doc without the guide facts is refused outright.
        self.assertTrue(guide_label_problems(minimal_doc()))
        # A fact whose tag loses its label word, its date or its guide URL.
        fracture = place_text(self.text, "Currency", "Fracturing Orb")
        self.assertIn("(label: community guide;", fracture)
        for old, new in (("(label: community guide;", "(community guide;"),
                         (GUIDE_DATE, "2026-10-29"),
                         (MOBALYTICS, "https://example.invalid/")):
            broken = self.text.replace(fracture, fracture.replace(old, new))
            self.assertTrue(guide_label_problems(broken), old)
        # A guide listed under ## Sources without the date, or not at all.
        sources = section(self.text, "Sources")
        self.assertTrue(guide_label_problems(self.text.replace(
            sources, sources.replace(GUIDE_DATE, "2026-10-28"))))
        self.assertTrue(guide_label_problems(self.text.replace(
            sources, sources.replace(MAXROLL_HOW_TO, "https://maxroll.gg/poe2"))))
        # The principles section moved after ## Gaps, and the alloy gap restored.
        guide = "\n## " + GUIDE_SECTION + "\n"
        head, tail = self.text.split(guide, 1)
        principles, rest = tail.split("\n## Gaps\n", 1)
        self.assertTrue(guide_label_problems(
            head + "\n## Gaps\n" + rest + guide + principles))
        self.assertTrue(guide_label_problems(self.text.replace(
            "## Gaps\n", "## Gaps\n\n- **Alloys and the crafted limit.** Open.\n", 1)))


if __name__ == "__main__":
    unittest.main()
