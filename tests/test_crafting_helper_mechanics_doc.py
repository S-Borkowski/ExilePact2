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
- corruption, Vaal Orb included, is out of scope rather than described.

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


if __name__ == "__main__":
    unittest.main()
