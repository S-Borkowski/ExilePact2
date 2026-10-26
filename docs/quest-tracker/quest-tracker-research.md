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
  seconds. The tracker applies no time window: every marker ends the
  session (held events are dropped and counted, the current area is
  cleared), so a launch's second and third markers each end an empty
  session, which changes nothing.
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

### Town dialogue

What the log records of talking to the NPCs in a town, measured 2026-10-25
over the owner's whole log with `quest-tracker-replay scan --speakers` and a
research script kept outside the repository that splits each town into
visits (an area entry to the next one). Counted are lines from the town's
own NPCs; chat, client lines and other players are left out.

| Town | Visits | Visits with an NPC line | NPC lines | Speakers |
|---|---|---|---|---|
| Clearfell Encampment (`G1_town`) | 100 | 44 | 140 | Una 51, Finn 49, Renly 34, Mortimer 6 |
| The Ardura Caravan (`G2_town`) | 103 | 42 | 139 | Zarka, Shambrin, Risu, Balbala |
| Ziggurat Encampment (`G3_town`) | 77 | 37 | 107 | Servi, Alva, Doryani, Oswald |
| Kingsmarch (`G4_town`) | 91 | 70 | 353 | Rog, Jacob, the Town Crier, Dannig, Tavakai and others |

- **What the lines are** (measured). All 44 distinct lines of Una, Finn,
  Renly and Mortimer in Clearfell Encampment are vendor and gambling lines
  ("Here's what I've got.", "Feelin' lucky?", "I will treasure it.") or
  Mortimer's farewell warning. None is a quest's talk, and none of the four
  speakers is on the quest data's speaker allowlist.
- **Live 1** (measured, `Client.txt` from offset 125256687 to the end of that
  session, 11:12Z-11:45Z on 2026-10-25). The character was in Clearfell
  Encampment for one visit, 3 min 35 s, between the level-up to 2 and the
  entry to Clearfell, and the person talked to the NPCs there. The log
  recorded no NPC line in that visit: only chat and client lines. The talk
  happened in dialogue windows, whose text was not observed in the log (see
  "What poe2db step text matches"). This is one visit: "not observed", not
  "never logged".
- **Live 3** (measured 2026-10-25, `Client.txt` from 21:04:27 to 21:07:20
  local time, read-only). The player was in Clearfell Encampment for one
  visit of about 3 min, and picked up Treacherous Ground from Renly there
  (the owner's report). The visit holds no NPC line: only chat, client and
  scene lines (the area entry, its name, the loading screen) and two
  window-focus lines, up to the entry to Clearfell. This is the second
  measured town visit with talks, after Live 1's, and again the talk was
  not observed in the log. Consequence for the data: Treacherous Ground's
  unlock gets the Clearfell Encampment entry (`G1_town`) beside the
  Clearfell entry, the earliest evidence of the pickup and the signal
  Finding the Forge, also Renly's, already unlocks on; that the town entry
  stands for the talk is inferred, not measured.
- **Consequence for the data.** A talk step cannot have a signal of its own.
  A town NPC's greeting is not evidence of a quest talk, so none is a step
  signal. A talk step is completed by the next signal the log records that
  the game produces only after it, where poe2db's step order or every own
  character's history in the log supports that order; the quest's `notes`
  then say `inferred` and why. Otherwise the step stays a tick.
- **Orders measured for Act 1** (2026-10-25, the own characters that started
  at The Riverbank; 7 reached Clearfell, 6 went on to the Cemetery):
  - Every one entered Clearfell Encampment before Clearfell (`G1_2`) and
    stayed 83 to 363 s on that first visit. Reaching Clearfell (talk to
    Renly, take his reward) is inferred done on entering Clearfell. A player
    who leaves without the talk is shown done too early.
  - Every one entered the Cemetery of the Eternals (`G1_7`) only after The
    Rust King's lines and a later visit to the encampment; The Mysterious
    Shade's steps put Una's last talk of Secrets in the Dark before she clears
    the Grim Tangle's roots. Secrets in the Dark is inferred done on entering
    the Cemetery. The Grim Tangle (`G1_6`) is no such signal: 3 of the 6
    entered it before the Rust King.
  - One of the 6 went from the Cemetery to the Hunting Grounds without a
    visit to the encampment, so the Hunting Grounds do not imply bringing
    the ring to Una (The Mysterious Shade stays a tick). 5 of 6 went from the
    Executioner to the Manor Ramparts without one, so the Manor does not
    imply the talks with Leitis (The Trail of Corruption stays a tick).
  - Talk steps followed by a step the log records (Finding the Forge's
    "Speak to Renly" before Ogham Village, Secrets in the Dark's "Ask Una for
    help" before the Red Vale) need no change: the engine takes the highest
    step reached, so the later signal completes them. The reward talks of
    Treacherous Ground and Finding the Forge have nothing after them and stay
    ticks.

### Rewards after a boss kill

How long after a boss's last voiced line its reward line comes, measured
2026-10-25 over the owner's whole log, per own character, from the boss's
last line in the area to the `has received` line:

| Quest | Gaps (measured) |
|---|---|
| Beira of the Rotten Pack | 7, 8, 9, 19, 36 and 44 s in Clearfell (`G1_2`); 134 and 465 s in Mud Burrow (`G1_3`); none in Live 3 |
| Blackjaw, the Remnant | 5 to 21 s |
| Lythara, the Wayward Spear | 10 to 15 s |
| The King in the Mists | 7 to 37 s |
| Ignagduk, the Bog Witch | 14 to 30 s |

- **The reward never comes with the boss's last line** (measured). No gap
  is zero, and Beira's reward comes up to almost eight minutes later and,
  twice, in the next area. That last line is not the kill, though: for
  Beira and BlackJaw it is a combat bark, voiced during the fight, so a gap
  of 5 to 36 s fits the rest of the fight just as well as a player action
  after it. The gaps alone do not tell "the reward comes on a later
  action" from "the reward comes at the end of the fight". Only two things
  in the log do: Live 1 (465 s, across an area change, below) and
  Lythara's death cry.
- **Lythara's death cries** (measured). Her gaps (10 to 15 s) are timed from
  a death cry, not a bark, so for her the line does mark the end of the
  fight, and the reward still came seconds later. Two more of her death
  cries in the log have no reward line after them at all. A reward granted
  at the kill would follow every death cry; these two fit a reward that
  waits on something the player did not do, though why none came is not
  established.
- **Live 1** (the owner's observation, 2026-10-25). Beira's last line was at
  12:31:50 local time and her reward line at 12:39:35 (465 s); no line in
  between concerns the quest (the first 100 s hold only chat and window focus
  lines). The owner saw the reward come
  when the item Beira dropped was used, not at the kill. The log records
  nothing of picking the item up or using it.
- **Beira's lines and her defeat** (measured 2026-10-25, the whole log up
  to the end of Live 3, read-only, by a script kept outside the
  repository). 9 sessions hold her lines (73 in all). Her first line in
  each is "Sleep in ice, wake in snow. Sleep my pretties as you grow...";
  the rest are combat barks: "Be still!", "Invader!", "Feed!", "Swell!",
  "Rotten!", "My pets hunger!". No line marks her defeat (not observed):
  between her last line and the next area entry or the reward line the
  log holds only chat, window-focus and client lines, and once a level-up
  and a passive allocation. The positive control in the same reading is
  her own lines and the reward line, which it finds. The `+10% to
  [Resistances|Cold Resistance]` reward came 7 to 44 s after her last
  line in 6 sessions, in Clearfell, and after the player had left
  Clearfell in 2, in the Mud Burrow, about 2 and 8 minutes later. In
  Live 3 her only line was the first, at 21:12:51 local time; the player
  entered the Grelwood at 21:14:13, and no reward line came before the
  game closed.
- **That negative is not controlled.** No reading was set against a known
  kill time, and the owner says Beira was not defeated in the live
  sessions (2026-10-25), so the 9 sessions may not hold a defeat that
  could have been logged. Live procedure 4 measures her zone from the
  entry through the first encounter and the kill to leaving, against the
  person's clock times. Until then the tracker reads no defeat: by the
  owner's decision of 2026-10-25 her lines mark the find step, "Defeat
  Beira of the Rotten Pack" has no signal and the player ticks it, "Use
  the item it dropped" has none either, and the reward line finishes the
  quest in any area (Blackjaw and Lythara get the same split). No other
  campaign quest's reward in the quest data is `+10% to Cold Resistance`.
- **Blackjaw and Lythara** (inference). That their reward also follows the
  use of an item the boss dropped is read from the above, not observed in
  play: Blackjaw's gaps are timed from a bark and cannot separate the two
  readings, and Lythara's death cries point the same way as Beira but
  name no item. Whether either boss drops one is not established.
- **Consequence for the data.** Beira of the Rotten Pack, Blackjaw, the
  Remnant and Lythara, the Wayward Spear, whose steps ended at the fight and
  whose `done` is the reward line, get a last step with no signal (poe2db
  does not name the item): Beira's is "Use the item it dropped"; Blackjaw's
  and Lythara's is "Use any item left after the fight", whose text gives
  their measured gaps and says that an item is not established for them.
  Up to the Live 3 build the boss's line was the fight's signal, so the row
  showed the use as the step to do as soon as the boss spoke (Live 2 and
  Live 3); since the owner's decision above, the boss's line reaches the
  find step and the defeat is ticked. The reward line still finishes the
  quest. The King in the Mists (Ominous
  Altars) and Ignagduk (Tribal Vengeance) already have steps after the fight
  that lead to the reward, and Abandoned Prison's reward comes from the
  Goddess of Justice in the prison, not from an item the Prisoner drops
  (poe2db, Abandoned Prison and The Prisoner pages, fetched 2026-10-25), so
  those three are unchanged.

### Character remarks

Whether the remarks the player's own character voices (a class's comment on
reaching a place or killing a boss) leave a line in the log. Measured
2026-10-25 over the owner's whole log (823,584 lines, up to the end of Live 2
at 18:38 local time), read-only, by research scripts kept outside the
repository, and against poe2db.

- **poe2db lists them** (measured). The poe2db tool's cache (the index and
  74 campaign quest pages) holds no class remark: the class words in it are
  NPC names and NPC dialogue. Three pages were then fetched by hand, at least
  4 s apart, and kept outside the repository. Each class page has a "Text
  Audio" table of the class's voiced lines, each with a key naming the
  moment that plays it: the Mercenary's has 2,354 entries (1,372 distinct
  texts), the Warrior's 2,048 (1,275). Many keys name a quest moment, such
  as `MillerKillComment`, "I've fought some big buggers before, but...
  blimey." (https://poe2db.tw/us/Mercenary, fetched 2026-10-25). The other
  classes' pages were not fetched.
- **No speaker is a class or a character's voice** (measured). None of the
  log's 249 distinct speaker-shaped prefixes equals one of the 14 class names
  its level-up lines name (ascendancies included). Three contain a class
  word: two are NPCs (Ignagduk, the Bog Witch; Dannig, Warrior Skald) and the
  third is a one-word name no level-up names. Eleven lines have a speaker
  that a level-up line names as a character, and by their text all eleven
  are typed local chat (greetings, trade and help requests), not voiced
  lines.
- **No class remark was found in the log** (measured). Every distinct
  Mercenary and Warrior text, normalised for case and spacing, was compared
  with every INFO message that is not chat, with a speaker or without one.
  10 Mercenary and 26 Warrior texts equal a logged line and 2 more Warrior
  texts are part of one, but every such line has an NPC or boss as its
  speaker and is a short bark many voices share ("Look out!", "Burn!") or a
  part of that NPC's own sentence. None is logged without a speaker or under
  a character's name, though both classes level up in the log.
- **Positive control** (measured). The same match, run with Una's "Text
  Audio" table (https://poe2db.tw/us/Una, fetched 2026-10-25, 87 distinct
  texts), finds 19 of her texts in the log, 18 of them under the speaker
  Una. The instrument finds a voiced line when the log carries one.
- **Speakerless lines near quest moments** (measured). Within 10 s of an
  area entry or an NPC line, the only speakerless INFO lines without a `: `
  prefix are client housekeeping (bundle requests, login server, start-up
  precalculations). Of the `: ` notices that are not the known event lines,
  50 shapes come in a campaign area: trade, AFK, item and door messages, the
  Vaal chants in `G3_16` (the same lines with three different classes), Act 4
  sailing orders such as "Half sail!" beside Makoru's lines, and one "Oh my,
  what is this?" in `G3_5`. None of them is in the Mercenary's or the
  Warrior's table; who voices the sailing orders and that one line is not
  established.
- **Live 2** (measured, `Client.txt` from 17:30 to 18:40 local time on
  2026-10-25: 3,147 lines, 562 of them chat). The only line shapes not seen
  earlier in the log are the new character's own event lines (level-ups, a
  death, a reward) and the loading screen of a first-visited area. The new
  character, a Mercenary, killed The Bloated Miller (the Miller's last line
  at 17:42:57, the level-up to 2 one second later), the moment poe2db's
  `MillerKillComment` names; the 60 s after it hold one window-focus line
  and nothing else. Whether the person heard the remark was not asked.
- **Live 3** (measured 2026-10-25, Live procedure 3 step 16, read-only by a
  script kept outside the repository). The person named four remarks of
  their character by the minute they heard them: near the Wounded Man, at a
  skill gem, on seeing The Bloated Miller and after killing him. The window
  read was `Client.txt` from 20:48:00 to 20:53:39 local time, which covers
  all four minutes and ends with the game closing. Besides chat it held 26
  lines: 13 window-focus lines, 6 of The Bloated Miller, 3 of the Wounded
  Man, 1 of Mortimer, the level-up to 2 (four seconds after the Miller's
  last line) and 2 client lines. None of them is a remark. The Wounded Man's
  three lines, which the person heard there, are the positive control: the
  log carried the NPC lines spoken in the same minutes. Over the whole Live 3
  session (20:40 to 21:18 local) every voiced speaker was an NPC or a boss
  (The Bloated Miller, the Wounded Man, Mortimer, Beira of the Rotten Pack,
  Una).
- **Not observed** (a negative, not proof of absence): a line carrying the
  own character's remark, under any speaker or none, anywhere in the log,
  including two sessions with a person listening for them (Live 2's Miller
  kill, Live 3's four remarks). That the client never logs them is not
  established.
- **Consequence for the data.** None: this is research only and the tracker
  is unchanged (the owner, 2026-10-25: "if not we can skip it"). A remark
  cannot be a step signal while it is not observed in the log.

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
- **A new character's first area** (measured 2026-10-25, over the owner's
  whole log, read-only). The log holds 12 entries to The Riverbank
  (`G1_1`), and every one is generated at area level 1 (`Generating level
  1 area "G1_1"`): level 1 is the area's level, not a sign of a new
  character. Eight of the entries, one per character, are followed in the
  same session by that character's first level-up, to level 2, 23 to 110
  lines later, with no other level-up in between; the name in that
  level-up never joined or left the area, and 7 of the 8 names later
  appear in a `has received` line. Between those eight entries and their
  level-ups no `[WARN] … unknown build` line or other client line names
  the character: in the Live 1 session (11:12Z-11:19Z on 2026-10-25) the
  only lines there are NPC lines (Wounded Man, Mortimer, The Bloated
  Miller), chat and client lines.
- **A relog inside The Riverbank** (measured 2026-10-25, same log). The
  other four entries re-generate one instance (one seed) in four
  consecutive sessions, each ended by a session marker before any
  level-up. All four are the same character, still at level 1: its first
  level-up followed its fifth entry to that instance, in the next session
  (that fifth entry is one of the eight above). So an existing character
  re-entering `G1_1` was observed, four times, each a relog of a
  character that had not yet levelled. In each of those four sessions a
  `[WARN] Character <name> has unknown build of path: None` line named the
  character 1 to 2 s after the entry. That line is not a general signal:
  the whole log holds 27 of them, all for this one character, in five
  sessions; none followed that character's fifth entry (the one before its
  level-up), and none came for any of the 7 other new characters. A `G1_1`
  entry alone therefore does not tell a new character from a relog. A
  character past level 1 entering `G1_1` was not observed up to Live 2;
  Live 3 observed it (below).
- **The Riverbank's seed** (measured 2026-10-25, one player's whole log up
  to the end of Live 3, read-only, by a script kept outside the repository).
  The log holds 16 entries to `G1_1`, from 2025-08-29 to 2026-10-25, and
  they generate 10 distinct seeds, that is 10 instances. Each instance's
  first entry is followed by its character's first level-up (for the
  instance relogged four times, after its fifth entry; for Live 3's, after
  its second). The other 6 entries re-enter an earlier entry's instance,
  with the **same seed**, and each is the same character as that instance's
  earlier entries: four in consecutive sessions on 2026-05-29 (the relogs
  above, still at level 1) and two in Live 3, one at level 1 and one after
  the character's level-up to 2, which is the past-level-1 relog not
  observed before. The 10 first level-ups name 10 different characters, so
  no new instance was a stored character coming back (re-entries with a new
  seed: 0 observed). Each re-entry came right after a session marker; 2 of the 10 new instances followed another area in the same
  session (after `G2_town` on 2025-08-30, after `G1_town` on 2025-12-12).
- **The name line on a relog** (measured, same reading). The `[WARN]
  Character <name> has unknown build of path: None` line came within 2 s
  of 4 entries, all to the 2026-05-29 instance: its first entry and the
  next three. So it came after 3 of the 6 re-entries (the fourth relog on
  2026-05-29 had none) and after neither of Live 3's two. This corrects an
  earlier count of "four relogs with the line", which took that instance's
  first entry for a relog.
- **`Requesting to complete all tutorials due to the client setting`**
  (measured, same reading). It followed 8 of the 10 new instances, every
  one since 2025-09-06: in 7 of them 0 to 2 s after the entry, in the
  2025-09-06 one 57 s after. It followed none of the 6 re-entries, and once
  it came 1 s after a hideout entry instead. It is a client-setting line
  that names no character, and the tracker does not read it.
- **The entry line names no character** (measured). All 16 entries are
  `Generating level 1 area "G1_1"`: the number is the area's level, not the
  character's, and the entry line names no character. Live 3's relog after
  the level-up to 2 was generated at level 1 as well.

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
session and never written anywhere. The new character from The Riverbank
(below) is the one exception to the pin and the hold: while it is open, the
session's events go to it, and a pin its entry released waits for it.

From the Riverbank counts (again a design choice resting on them, not a
measurement): a level-1 `G1_1` entry starts a new, unnamed character once
the relog window below closes without a relog, and its events go to it
instead of the hold, so Act 1 progress shows within seconds;
it is never listed among the characters. The first level-up after it,
from a name not excluded as a party member, names it: that name becomes
own and active with no pick, takes the progress and is stored. The entry
releases a pin, and the named character is pinned in its place, because
the pin a player sets on one character would otherwise hide every later
new character.

The relog rule (a design choice, the owner's of 2026-10-25, resting on the
27 lines and the four sessions above): the `[WARN] Character <name> has
unknown build of path: None` line is read, in that exact shape only, as a
character's name. Because that line comes after the entry, the tracker
waits before it decides (the owner's choice after launch 7, "wait, then
decide"): a level-1 `G1_1` entry, while no new character is open, opens a
relog window of 5 s and changes
nothing else, with no note, the pin and the active character kept, and
the window's events waiting. Such a line no more than 5 s after the entry,
before any other area entry, naming a character the tracker already
stores, makes the entry a relog: the window closes having changed nothing,
no new character starts and no pin is released, that character is active,
and the window's events go where routing sends them. Any other such line
changes nothing. The window otherwise closes on another area entry, a
line more than 5 s after the entry, a level-up, a session marker, the
player's pin (which decides it for the pinned character), or, on
a quiet log, the overlay's clock passing 5 s of log time after the entry
(at most about 7.5 s after it: 6 s counted from the log's whole-second
timestamps, up to 1 s of offset between a line's stamp and when it was
written, and two 250 ms polls); only then does the entry start (or resume) the new character and
release a pin. So a relog never shows a new character, even for a
moment. The 5 s bound is the measured 1 to 2 s with room to spare; a
2 s window was rejected for having none. Because the line came for one
character only, its absence proves nothing: a relog without it still
starts the new character once the window closes.

The relogs measured before Live 3 were of a character with no level-up
yet, so no name was stored for the rule to match. For that case the unnamed new character
is kept as one provisional record (its progress, its event count, the pin
it released, the seed of its `G1_1` instance, and whether it is open),
saved apart from the characters. A session marker parks it instead of
dropping its events, and gives the pin back; a later level-1 entry to the
**same** instance (same seed, as the four measured relogs had) resumes it,
and one to another instance replaces it, counting the replaced record's
events as dropped apart from those that came before any character. A
level-up naming it, a relog by the rule above or the player's pin closes
the record. Up to the Live 3 build, a known character past level 1
re-entering `G1_1` with no build line showed as "New character" until its
next level-up; Live 3 observed exactly that relog, with no name line.

The seed rule (a design choice of 2026-10-25, resting on the seed
measurements above: every one of the 6 re-entries had its instance's seed,
and none of the 10 new instances was a stored character's return): each
stored character keeps the seed of the Riverbank instance it was last
decided to be in, given to it when a decision puts it there (the new
character's seed passes to the name its level-up gives it, or to the
character a pin hands its progress to, and a relog decided by seed or by
the name line records that entry's seed; a routed event never records
one). A level-1 `G1_1` entry whose seed is that of exactly one stored
character, and of no parked new-character record, is that character's
relog, decided at the entry with no wait. Any other entry opens the relog
window above, and the name line keeps working inside it; two characters
with one seed fall back to the window. A stored character returning to
The Riverbank in a fresh instance (its old one closed) still starts a "New
character" that its next level-up hands back; that case was not observed
in the log.

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
  was measured. "auto (inferred)" in the table marks a completion that is a
  later area entry standing for steps the log does not record, such as a
  talk ("Town dialogue" above); the quest's `notes` say why.
- **Evidence**: `measured` when the completion signal is; `static` when the
  steps and areas come from poe2db's page alone; `not-established` when the
  page has no steps table either, so only the quest's name, act and rewards
  are known.
- **Worth**: `yes` for a permanent character stat, an ascendancy or the main
  story; `optional` for gems, lasting items and unlocks; `no` for story-only
  or currency-only side content.

Totals: 32 auto, 42 partial, 13 manual; 32 measured, 44 static, 11
not-established; 50 worth it, 14 optional, 23 not. No Act 5 quest uses a
measured signal (see "What poe2db step text matches").

**Secrets in the Dark and The Mysterious Shade** (measured 2026-10-25,
read-only over the owner's `Client.txt`, unless labelled otherwise). Una's
voiced lines in the Grelwood (`G1_4`) and the Grim Tangle (`G1_6`), in the
order they came; 7 sessions of the log up to Live 2 hold her Grelwood
lines, and they came in the same order in each of them. Live 3 is an
eighth, with her first line only:

| Area | Una's line | Sessions | Where it falls |
|---|---|---|---|
| `G1_4` | "What happened here...?" | 8 | her first Grelwood line each time, when she is summoned at the Tree of Souls |
| `G1_4` | "I'll wait here. The old magicks will protect me." | 4 | 2 to 8 s after it (from December 2025 on; 8 s in Live 2) |
| `G1_4` | "Good. Now the others!" | 6 | after that session's Red Vale (`G1_5`) visits |
| `G1_6` | "The old magicks still thrive! The way is open. Please, hurry." | 6 | after "Good. Now the others!" each time |

- Secrets in the Dark was *available* from Clearfell (`G1_2`) and in
  progress only from the Grelwood in Live 2 (measured): its first step's
  signal is the Grelwood entry, and Renly gives the quest in a talk the
  log does not record. The data's `start` signal, Reaching Clearfell done
  (which fires on the Clearfell entry), makes it in progress from there.
  That start is **inferred**, not measured: the Clearfell entry is the
  earliest evidence of the talk, not the talk.
- The wait line follows the talk, not the summon (measured 2026-10-25). In
  Live 2 the person had talked to Una ("even after talking to una", Live 2
  capture), and the wait line came 8 s after "What happened here...?". In
  Live 3 the person summoned her without the talk: "What happened
  here...?" came at 21:16:26 local time and the game closed about 2.5 min
  later with no wait line. Sessions before December 2025 never have it.
  So "What happened here...?" is the signal of "Search the Grelwood"
  (summoning her ends the search), and the wait line alone is the signal
  of "Ask Una for help" (**inferred**: it follows the talk, two sessions
  read against what the person did). Where the wait line never comes, the
  Red Vale entry completes the steps before it, as the engine takes the
  highest step reached. Up to the Live 3 build both first lines were the
  signals of "Ask Una for help", so a summon alone showed "Find the Red
  Vale" (Live 3). That "Good.
  Now the others!" is "Use the Rune Spikes" is **inferred** from its place
  after the Red Vale; that the Grim Tangle line is The Mysterious Shade's
  "Speak to Una about the roots" is **inferred** from the step text.
- The Mysterious Shade's "Find the Grim Tangle" carried the Grelwood entry
  as its signal, which fires on the way to Secrets in the Dark, before The
  Mysterious Shade unlocks; in Live 2 it showed that quest in progress
  that early (measured). The step now has no signal.
- Una's town lines match no signal, because each of her signals names its
  area and its exact line.

**Beira of the Rotten Pack, Blackjaw and Lythara** (the tracker's reading
of "Rewards after a boss kill" above, the owner's decision of 2026-10-25).
Beira's steps are "Reach Clearfell" (the `G1_2` entry, measured), "Find
Beira of the Rotten Pack" (any of her lines in `G1_2`, measured: her first
line comes when she is met), "Defeat Beira of the Rotten Pack" (no signal:
no line marking her defeat was observed, a negative not yet set against a
known kill, so the player ticks it), "Use the item it dropped" (no signal:
the use is not logged) and `done`, the `+10% to Cold Resistance` reward line
in any area (measured in Clearfell and, in 2 sessions, in the Mud Burrow;
that it may come in any area where the item is used is **inferred** from
those two). So her lines leave the defeat current whatever area comes next,
the tick shows the use, and the reward finishes the quest from either step.
No leave or timer inference marks the defeat (rejected by the owner,
2026-10-25). Blackjaw, the Remnant and Lythara, the Wayward Spear get the
same split, by analogy (**inferred**): their lines reach "Find …",
"Defeat …" has no signal, the "Use any item left after the fight" hint is
unchanged. Evidence an older build stored for a boss line on the fight step
matches no compiled signal now and is ignored, so such a character shows the
step its other evidence reaches.

**Treacherous Ground** (the tracker's reading of "Town dialogue" above).
Its unlock is the Clearfell Encampment entry (`G1_town`) or the Clearfell
entry (`G1_2`), whichever comes first. The pickup from Renly is a talk the
log does not record (two measured town visits); the town entry, where Renly
stands and on which Finding the Forge already unlocks, is the earliest
evidence of it. That the town entry stands for the pickup is **inferred**
(2026-10-25); a player who enters the town without talking to Renly sees
the quest available early.

**Find-it tips** (not a measurement). Five steps carry the owner's own
"how to find it" text in their `tip` field (2026-10-25): Slay the Bloated
Miller, Find the Mud Burrow, Find Beira of the Rotten Pack, Travel to the
Grelwood and Search the Grelwood. They are the owner's advice, not read
from the log or from poe2db, and change no signal.

| Act | Quest | Worth | Detection | Evidence |
|---|---|---|---|---|
| 1 | Reaching Clearfell | yes | auto (inferred) | measured |
| 1 | Treacherous Ground | optional | partial | static |
| 1 | Secrets in the Dark | yes | auto (inferred) | measured |
| 1 | The Mysterious Shade | yes | partial | static |
| 1 | Finding the Forge | optional | partial | static |
| 1 | The Lost Lute | yes | auto | measured |
| 1 | Sorrow Among Stones | yes | partial | static |
| 1 | Ominous Altars | yes | auto | measured |
| 1 | The Trail of Corruption | yes | partial | static |
| 1 | The Mad Wolf of Ogham | yes | auto (inferred) | measured |
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
