# Quest tracker research: what Client.txt records about the campaign

This note argues out which campaign quest steps leave a trace in the game's
`Client.txt` log, and therefore which steps the quest tracker can advance on
its own and which the player has to tick by hand. The facts about the log
itself that every mod can use are folded into
[`docs/RUNTIME_DATA_MODELS.md`](../RUNTIME_DATA_MODELS.md) § "Client log
(Client.txt)"; this note keeps the counts, the negative results and the
reasoning.

Every claim carries one of three labels:

- **measured**: counted in a real log or a real page, with the date;
- **static reading**: read from a file or a page without watching the game do
  it (for example, which quest a reward belongs to according to poe2db's
  community table);
- **not established**: neither measured nor read; a guess the tracker must not
  rely on.

A negative is written as "not observed", never as "does not happen": every
count below comes from one player's log, and a line the client writes only in
a situation this player never reached would be missing from it too.

## Sources and method

- **The log** (measured 2026-10-24). The owner's `Client.txt`, under
  `C:\Program Files (x86)\Grinding Gear Games\Path of Exile 2\logs\`, read
  read-only by scripts kept outside the repository: 125,140,938 bytes, 818,412
  lines, 2025-08-04 to 2026-10-19, CRLF line endings, UTF-8. It holds six own
  characters' campaign runs plus party play and endgame. `LatestClient.txt`
  beside it (1.3 MB) holds only the latest launch: one session marker, and it
  is rewritten on each launch. The location of the log for a Steam install is
  not established.
- **poe2db** (measured 2026-10-24). 13 requests, at least 3 s apart, to
  `https://poe2db.tw/us/Quest` and eleven quest pages; the pages stayed in a
  scratch folder and are not tracked. `robots.txt` reads `User-agent: *` and
  disallows only `/*/marked` and `/*/markdown_history`.
- **poe2db, every campaign page** (measured 2026-10-25). `py -3 -m
  tools.quest_tracker.poe2db fetch` cached the index and 74 campaign quest
  pages (default 3 s delay) outside the repository; `extract` drafted them
  and `coverage` measured their dialogue against the log. The replay tool's
  `scan` and a research script kept outside the repository, both run
  read-only over the same log, gave the per-area names, speakers, reward and
  points lines that `apps/quest-tracker/data/quests.json` is curated from
  (see "Per-quest detection").
- **The client executable** (static reading). Its ASCII and UTF-16 strings
  were searched for the reward and level-up message templates; see "Negative
  results". No decompiler or disassembler was used for this feature.
- **What is quoted here.** Counts, line templates with the variable parts
  replaced by `<placeholders>`, area codes, area and NPC names, and short
  game text (reward and NPC lines), which the owner allows to be quoted. No
  chat line, no player or account name, and no log line is reproduced whole.

## Client.txt line format

- **Line shape** (measured). `YYYY/MM/DD HH:MM:SS <ms counter> <hex> [<LEVEL>
  Client <pid>] <message>`. The levels seen are `INFO`, `DEBUG`, `WARN` and
  `CRIT`. The millisecond counter and the hex field carry nothing the tracker
  needs.
- **Continuation lines** (measured). 2,282 lines carry no timestamp; they are
  the continuations of multi-line warnings. The parser ignores them.
- **Session marker** (measured). `YYYY/MM/DD HH:MM:SS ***** LOG FILE OPENING
  *****`, 211 times. One launch can write it two or three times within a few
  seconds, so the tracker treats markers a few seconds apart as one session.
- **Client version** (measured). `[HTTP2] User agent: PoE
  poe2_production/tags/<version> Windows x64`. Versions 4.3.0 through 4.5.3
  appear, then 0.5.5 on 2026-10-19, with only a patch check after it. Why the
  numbering went down is not established.
- **Encoding and size** (measured). UTF-8, CRLF. The file only grows; at
  125 MB after fourteen months, a reader must start from the end or from a
  remembered offset, never re-read the whole file on each poll.
- **Client-internal lines that look like speech** (measured). Lines of the
  form `<word>: <text>` are not only NPC speech: `Queue file to download:
  ...` appears 258,476 times and `Tile hash: ...` is frequent too. Only an
  allowlist of known speakers can tell an NPC line from these.

## Quest signals in the log

What follows is everything in the log that moves with the campaign. All
counts are measured on 2026-10-24 unless the line says otherwise.

- **Area entered.** A `DEBUG` line `Generating level <level> area "<code>"
  with seed <n>`, 3,594 times: exactly as many as `Connecting to instance
  server` lines, one per instance join. Towns and hideouts have seed 1. The
  area code is the stable signal.
- **Area display name.** An `INFO` line `[SCENE] Set Source [<name>]`, the
  first one 3 to 16 lines after `Generating level` (4 lines after it in 3,141
  of 3,594 joins), following a `[SCENE] Set Source [(null)]`. The values
  `(null)`, `(unknown)` and `Act <n>` are not area names. One name carries a
  trailing space (`Sel Khari Sanctuary `), so names are stripped.
- **Area codes seen.** `G1_*` to `G4_*`, with `G<n>_town` for each act's town;
  the interludes `P1_*` (Ogham), `P2_*` (Khari) and `P3_*` (Mount Kriar); and
  `G_Endgame_Town`. No `G5_*` or `G6_*` code was observed in this log.
- **NPC lines.** `INFO <Speaker>: <text>` with no prefix. Logged speakers
  include Renly, Una, Finn, The Hooded One, Delwyn, Zarka, Sekhema Asala,
  Shambrin, Alva, Servi, Doryani, Dannig, Tujen, Gwennen, Hinekora, Rog, Kanu,
  Tavakai, Makoru, Jado, Navali, Rakiata and Maata, and bosses (The King in
  the Mists, Count Geonor, Beira of the Rotten Pack, Kabala, Constrictor
  Queen, Ignagduk, the Bog Witch, BlackJaw, Lythara, The Wayward Spear and
  others). What is logged is short voiced lines: vendor greetings, remarks on
  entering a zone, boss lines. The dialogue window's text was not observed (see
  "What poe2db step text matches"). A boss's voiced line in its own area is
  the closest thing the log has to "this fight happened".
- **Chat** (to be dropped). Other players' chat has the NPC shape with a
  prefix: `#` (41,249), `$` (1,823), `%` (425) and `@` (309). `&` (guild) was
  not observed in this log but is dropped too. Local chat has no prefix and
  so the same shape as an NPC line; this is why the speaker allowlist is
  needed.
- **Permanent rewards.** `: <character> has received <stat>.`, with markup
  `[Group|Shown]` or `[Shown]` in which only the part after `|` is shown.
  Seen: `+30 to [Spirit|Spirit]`, `+10% to [Resistances|Cold Resistance]`,
  `+20 to maximum Life`, `+1 [Charm] Slot`, `30% increased [Charm] Effect
  Duration`, `30% increased Life Recovery from [Flask|Flasks]`, `5% increased
  maximum Mana`, `+5 to [Strength|Strength]`, `+5% to all
  [ElementalDamage|Elemental] [Resistances]`, `15% increased Global
  [Defences]`, `3% increased Movement Speed`, `25% increased
  [StunThreshold|Stun Threshold]`, `25% increased Mana Regeneration Rate`,
  `30% increased Global [Armour], [Evasion] and [EnergyShield|Energy
  Shield]`. When a swappable boon is changed, `: <character> has lost
  <stat>.` appears (3 times).
- **Passive points.** `: You have received <n> Passive Skill Points.` (53)
  and `: You have received <n> Weapon Set Passive Skill Points.` (53). They
  always occur in the same areas with the same counts, and they name no
  character. Atlas, Genesis Tree and other league point lines are endgame and
  out of scope.
- **Level-ups.** `: <character> (<class>) is now level <n>`, for party
  members as well as own characters.
- **Other character lines.** `: <character> has been slain.` (459, party
  members included), `: <name> has joined the area.` (274) and `: <name> has
  left the area.` (256).
- **Speakerless campaign lines.** Vaal chants in Aggorat (`G3_16`) and the
  boat calls `Half sail!` and `All stop!` in act 4. `Open the eye of the
  storm.` and ten lines like it occur only in endgame `Map*` areas, and
  `Collected a Silver|Bronze|Gold Key` belongs to the Sanctum; neither is
  campaign.

**Reward lines by area.** The area and the line are measured; which quest the
reward belongs to is a static reading of poe2db's community table. Each row
was seen 1 to 6 times, from six characters.

| Area (code) | Reward line | Quest (static reading) |
|---|---|---|
| Clearfell (G1_2) | +10% Cold Resistance, Beira speaking | Beira of the Rotten Pack |
| Hunting Grounds (G1_11) | passive points | The Crowbell |
| Clearfell Encampment (G1_town) | passive points | Una's Lute to Una |
| Freythorn (G1_12) | +30 Spirit, the King in the Mists speaking | The King in the Mists |
| Ogham Manor (G1_15) | +20 maximum Life | Candlemass |
| Keth (G2_4_1) | passive points, Kabala speaking | Kabala |
| Valley of the Titans (G2_6) | Charm Slot or Charm Effect Duration | relic altar |
| The Ardura Caravan (G2_town) | passive points | Final Letter to Shambrin |
| The Spires of Deshar (G2_9_2) | +10% Lightning Resistance | Sisters of Garukhan shrine |
| Jungle Ruins (G3_3) | passive points | Mighty Silverfist |
| The Azak Bog (G3_7) | +30 Spirit, Ignagduk speaking | Ignagduk |
| Jiquani's Machinarium (G3_6_1) | +10% Fire Resistance, BlackJaw speaking | Blackjaw |
| Ziggurat Encampment (G3_town) | Stun Threshold or Mana Regeneration | Servi's venom draught |
| Aggorat (G3_16) | passive points | Sacrificial Heart |
| Infested Barrens (G3_2_1), Isle of Kin (G4_1_1), Volcanic Warrens (G4_1_2) | passive points, once each | not established |
| Eye of Hinekora (G4_4_1) | 5% maximum Mana | Navali's Rest |
| Halls of the Dead (G4_4_2) | +5% to one resistance or +5 Strength | Trial of Ngakanu (a choice) |
| Trial of the Ancestors (G4_4_3) | passive points | Hinekora's trial |
| Abandoned Prison (G4_5_1) | 30% Life or Mana Recovery from Flasks | Goddess of Justice |
| Kingsmarch (G4_town) | Armour, Evasion and Energy Shield (Rog); passive points (source not established) | Great White One |
| Wolvenhold (P1_5) | passive points | Oswin |
| The Khari Crossing (P2_1) | 5% maximum Life | Skullmaw Stairway |
| Qimah (P2_6) | all Elemental Resistances, Global Defences or Movement Speed | Qimah boons |
| The Khari Bazaar (P2_Town) | passive points | Worm and Scorpion |
| Kriar Village (P3_2) | +40 Spirit, Lythara speaking | Lythara |
| The Glade (P3_Town) | passive points | Yeti, or the interlude's completion (not established which) |

**What this means for the tracker** (static reading of the above). A quest
can be advanced automatically on three kinds of evidence: entering an area
the quest names (the area code, or the `[SCENE]` name), a voiced line from
an NPC or boss the quest names, and a reward line in the quest's reward area.
A reward line is the strongest of the three, because it only appears once
the reward is granted; a points line names no character and is attributed to
the active one. A quest whose steps happen inside a dialogue window or an
interaction leaves nothing in the log until its reward, and a quest with no
logged reward and no named area or speaker is manual: the player ticks it.

## Character detection

All of this is measured on 2026-10-24 unless labelled otherwise.

- Six names appear in `has received` lines, and none of the six ever appears
  in a `has joined the area` or `has left the area` line: these are the
  owner's own characters.
- Thirteen other names have level-ups (at levels 66 to 89), and every one of
  them also appears in a join or leave line: these are party members.
- One more name has a single level-up to level 2 and nothing else. Whether it
  was an own character deleted early is not established.
- No line marks character select or logging in as a character (not
  observed). In the whole log one character switch within a session was
  observed; the session marker therefore says nothing about which character
  plays.

From those counts the tracker's rules are (a design choice built on the
measurements, not a measurement): a name in a `has received` or `has lost`
line is own; a name in a join or leave line is not own for the rest of that
session; a level-up of an own character makes it active and records its class
and level; a level-up of any other name not excluded makes it active but
unconfirmed until a reward line or the player confirms it; events before any
active character are held for the first one that becomes active in the
session, and dropped and counted at session end if none does; a character the
player pins gets every event; `has been slain` never changes the active
character. Names seen only in join or leave lines are kept in memory for one
session and never written anywhere.

## What poe2db step text matches

- **What poe2db offers** (measured 2026-10-24). `https://poe2db.tw/us/Quest`,
  tab `#Quest`, lists 116 entries, each a link with its type (`Normal`,
  `Optional`, `Important`, `MasterMission`), act (`Act 1` to `Act 6`, and
  `Act 10`), reward and description. Some quests repeat per act (Ancient
  Beacons, The Runeseeker, The Trail of Corruption). Tab `#Quest_Markdown5`
  ("Community Wiki") holds per-act Zone, Action and Reward tables of the
  permanent rewards; it is community prose under the footer's CC BY-NC-SA 3.0
  notice, so worth reasons are written in our own words. Each quest page has a
  steps table: per step a title, text, `Area:` with the area name, optionally
  `NPC:` names, a dialogue topic and the dialogue split into snippets; then a
  reward table.
- **The campaign pages** (measured 2026-10-25). `fetch` cached 74 campaign
  quest pages for poe2db's Acts 1 to 6 (16 in Act 1, 13 in Act 2, 7 in Act
  3, 23 in Act 4, 7 in Act 5, 8 in Act 6), 709 steps in all. 11 of the 74
  pages carry no steps table (Back in the Bottle, Blank Runes, Cracks in the
  Earth, Greater Blank Runes, Infiltrating Oriath, No Rest for the Blighted,
  Pinned Down, Pleasure and Pain, Siren Pearl, The Blind Beast, The
  Forgotten Prisoner); `extract` drafts them with zero steps and names them.
- **poe2db's Act 6 is the interludes** (measured 2026-10-25). Every one of
  the 21 distinct `Area:` names on its Act 6 pages (Recruit the Ezomytes,
  Maraketh and Vaal, Clearing the Way, Howling Winds and the rest) is the
  `[SCENE] Set Source` name of a `P1_*`, `P2_*` or `P3_*` area in the log.
  **Act 5** (measured 2026-10-25): its pages name no `Area:` at all, and the
  log has no area for it; what its quests leave in the log is not
  established, so the data file ticks them by hand.
- **Dialogue against the log** (measured 2026-10-25, `coverage` over every
  fetched page). Over the 74 pages, 28 of which carry dialogue, 2 of 582
  dialogue snippets equal or contain a line the same NPC was logged saying,
  and 5 share a run of four words with one; 17 of the pages' NPCs have any
  line in the log at all. For example, none of Renly's 34 logged lines (10
  distinct, all vendor lines) is his step-6 dialogue in Reaching Clearfell
  (measured 2026-10-24). poe2db shows the dialogue window's text, and that
  text was not observed in the log.
- **What does match** (measured 2026-10-25). A step's `Area:` name equals the
  `[SCENE] Set Source` name of that area: 83 of the 85 distinct `Area:`
  names on the pages are names the log shows; the two that are not, Lightless
  Passage and The Well of Souls (both from the Act 2 quest The Well of
  Souls), are areas this log never entered. The step's NPC and boss names are the speaker names of
  the log, and the reward text matches the `has received` lines once the
  markup is stripped.
- **Consequence.** Step text is for display. Matching uses the area, the
  speaker names and the reward text; an NPC-line signal in the quest data
  comes from a line seen in the log, never from poe2db's dialogue.

## Per-quest detection

What `apps/quest-tracker/data/quests.json` claims for each campaign quest
(curated 2026-10-25). The file was built by hand from `extract`'s draft of
the 74 pages above and a read-only scan of the owner's log, not generated by
the tool; the 13 rows marked "community table" are permanent rewards that
poe2db's index lists in its community table without a quest page of their
own, sourced to the index page.

- **Detection**: `auto` when the quest's completion is a measured signal (a
  reward line, a points line or an area entry in the area the scan counted
  it); `partial` when only its unlock or steps are (area entries, a boss's
  voiced line), so the player ticks the end; `manual` when nothing about it
  was measured.
- **Evidence**: `measured` when the completion signal is; `static` when the
  steps and areas come from poe2db's page alone; `not-established` when the
  page has no steps table either, so only the quest's name, act and rewards
  are known.
- **Worth**: `yes` for a permanent character stat, an ascendancy or the main
  story; `optional` for gems, lasting items and unlocks; `no` for story-only
  or currency-only side content.

Totals: 30 auto, 44 partial, 13 manual; 30 measured, 46 static, 11
not-established; 50 worth it, 14 optional, 23 not. No Act 5 quest uses a
measured signal (see "What poe2db step text matches").

| Act | Quest | Worth | Detection | Evidence |
|---|---|---|---|---|
| 1 | Reaching Clearfell | yes | partial | static |
| 1 | Treacherous Ground | optional | partial | static |
| 1 | Secrets in the Dark | yes | partial | static |
| 1 | The Mysterious Shade | yes | partial | static |
| 1 | Finding the Forge | optional | partial | static |
| 1 | The Lost Lute | yes | auto | measured |
| 1 | Sorrow Among Stones | yes | partial | static |
| 1 | Ominous Altars | yes | auto | measured |
| 1 | The Trail of Corruption | yes | partial | static |
| 1 | The Mad Wolf of Ogham | yes | auto | measured |
| 1 | The Hunt Begins | no | partial | static |
| 1 | Blank Runes | optional | manual | not-established |
| 1 | Cracks in the Earth | optional | manual | not-established |
| 1 | Ancient Beacons | no | manual | static |
| 1 | Grim Portents | no | manual | static |
| 1 | Grave Tidings | no | manual | static |
| 1 | Beira of the Rotten Pack (community table) | yes | auto | measured |
| 1 | The Crowbell (community table) | yes | auto | measured |
| 1 | Candlemass, the Living Rite (community table) | yes | auto | measured |
| 2 | Earning Passage | yes | auto | measured |
| 2 | The City of Seven Waters | yes | partial | static |
| 2 | A Theft of Ivory | yes | partial | static |
| 2 | A Crown of Stone | yes | partial | static |
| 2 | The Forgotten Prisoner | no | partial | not-established |
| 2 | Back in the Bottle | no | partial | not-established |
| 2 | Tradition's Toll | yes | auto | measured |
| 2 | Ancient Vows | yes | auto | measured |
| 2 | Ascent to Power | yes | partial | static |
| 2 | The Trial of Time | no | partial | static |
| 2 | The Well of Souls | no | partial | static |
| 2 | Desert Omens | no | partial | static |
| 2 | Ash and Bone | no | partial | static |
| 2 | Kabala, Constrictor Queen (community table) | yes | auto | measured |
| 2 | Sisters of Garukhan Shrine (community table) | yes | auto | measured |
| 3 | Legacy of the Vaal | yes | auto | measured |
| 3 | The Slithering Dead | yes | auto | measured |
| 3 | Treasures of Utzaal | optional | partial | static |
| 3 | Tribal Vengeance | yes | auto | measured |
| 3 | The Trials of Chaos | yes | partial | static |
| 3 | Savage Wilds | no | partial | static |
| 3 | Blood Offerings | no | partial | static |
| 3 | Mighty Silverfist (community table) | yes | auto | measured |
| 3 | Blackjaw, the Remnant (community table) | yes | auto | measured |
| 3 | Sacrificial Heart (community table) | yes | auto | measured |
| 4 | The Runeseeker | optional | partial | static |
| 4 | The Search | yes | partial | static |
| 4 | The Blind Beast | optional | partial | not-established |
| 4 | Shrike Island | optional | partial | static |
| 4 | Land of the Kin | yes | partial | static |
| 4 | Utopia | yes | partial | static |
| 4 | Whakapanu Island | optional | partial | static |
| 4 | Abandoned Prison | yes | auto | measured |
| 4 | The Prisoner | no | partial | static |
| 4 | Tribal Medicine | yes | auto | measured |
| 4 | Dark Mists | yes | partial | static |
| 4 | Trial of the Ancestors | yes | auto | measured |
| 4 | Greater Blank Runes | optional | partial | not-established |
| 4 | Tawhoa's Test | yes | auto | measured |
| 4 | Tasalio's Test | yes | auto | measured |
| 4 | Ngamahu's Test | yes | auto | measured |
| 4 | Siren Pearl | optional | partial | not-established |
| 4 | Siege of Oriath | yes | manual | static |
| 4 | Hostile Takeover | optional | partial | static |
| 4 | Follow Tavakai | no | partial | static |
| 4 | Fate of the Vaal | no | partial | static |
| 4 | Blood in the Water | no | partial | static |
| 4 | Tidal Omens | no | partial | static |
| 4 | Navali's Rest (community table) | yes | auto | measured |
| 5 | The Siege of Oriath | yes | manual | static |
| 5 | Infiltrating Oriath | yes | manual | not-established |
| 5 | Plumbing the Depths | no | manual | static |
| 5 | Trarthan Gladiator | optional | manual | static |
| 5 | Pinned Down | optional | manual | not-established |
| 5 | Pleasure and Pain | no | manual | not-established |
| 5 | No Rest for the Blighted | no | manual | not-established |
| 6 | Recruit the Ezomytes | yes | partial | static |
| 6 | Recruit the Maraketh | yes | partial | static |
| 6 | Recruit the Vaal | yes | partial | static |
| 6 | Clearing the Way | yes | auto | measured |
| 6 | Howling Winds | yes | auto | measured |
| 6 | Shadow Over Ogham | no | partial | static |
| 6 | Sands of Ruin | no | partial | static |
| 6 | Death in the Ranges | no | partial | static |
| 6 | Oswin, the Dread Warden (community table) | yes | auto | measured |
| 6 | Skullmaw Stairway (community table) | yes | auto | measured |
| 6 | Qimah Boons (community table) | yes | auto | measured |
| 6 | Lythara, the Wayward Spear (community table) | yes | auto | measured |

## Negative results

Each of these is "not observed" in the log measured on 2026-10-24, not "does
not happen".

- No line names a quest, its start, a step or its completion. Among the
  `INFO` system messages there is no quest template at all.
- No line marks character select; one in-session character switch was seen
  in fourteen months.
- The dialogue window's text was not observed in the log; only short voiced
  lines were (see above).
- No `G5_*` or `G6_*` area code, and no area for poe2db's Act 5 quests
  (its Act 6 quests are the interludes, whose `P*_` areas the log has).
- No guild chat (`&`) line.
- The client executable's ASCII and UTF-16 strings hold quest-flag script
  names but none of the reward or level-up message templates (static
  reading). Those texts probably live in the game's data files, which were
  not read (not established).
- The source of the passive points in Kingsmarch, and of the single points
  lines in Infested Barrens, Isle of Kin and Volcanic Warrens, is not
  established.

## Patch sensitivity

- **Everything here is English-client text** (static reading). The parser
  reads the English templates (`has received`, `is now level`, `Generating
  level ... area`); another client language is out of scope and would not
  match.
- **Templates can change with a patch** (not established how often). The
  version jump from 4.5.3 to 0.5.5 on 2026-10-19 was followed only by a
  patch check, so no line from the newest client was measured. After a patch
  the parser fixtures and a run of the replay tool over a fresh log are the
  check: an unknown line is ignored, so a changed template shows up as quests
  that stop advancing, not as a crash.
- **Area codes are internal names** (static reading). They have been stable
  across the 4.3.0 to 4.5.3 range in this log (measured), but a campaign
  rework could renumber them; the quest data keys its signals on codes and
  keeps the display names beside them so a renumbering can be spotted.
- **poe2db changes independently of the game** (not established how often).
  The fetch tool records a source URL and fetch date per quest, so a stale
  entry can be found and refetched after a patch.
- **The quest data mixes two patches** (static reading). poe2db, fetched
  2026-10-25, describes the current patch, newer than the log's last
  gameplay on the 4.5.x client (4.5.3, the data file's `game_version`).
  Quests, steps and rewards come from the newer patch; every measured signal
  comes from the older one. Act 5 and Siege of Oriath have no gameplay in
  the log at all, and a reward or area the newer patch moved shows up as a
  quest that stops advancing, to be re-measured with the replay tool's
  `scan` over a fresh log.
- **The log file** (measured). It only grows and is shared with the client,
  which keeps writing it; the tracker opens it read-only and never moves,
  truncates or locks it.
