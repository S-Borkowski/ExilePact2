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
- **The millisecond counter** (measured once, 2026-10-26). It counts the
  milliseconds since the system started, not since the client or the log
  did. The line at 11:43:29 local time carried 90,865,203; counted from
  the machine's boot time, 2026-10-25T09:29:02.5Z, that second is
  90,866,500 ms, and a `GetTickCount64` reading taken at 11:28:54Z agreed
  within the line's one-second resolution. One comparison, so the reading
  rests on a single line; it gives an outside tool a clock it can set
  against the log without parsing the local time.
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
  (The paragraph above is kept as it was written on 2026-10-25; Live 4,
  below, now controls that negative with one kill.)
- **Live 4, the first controlled reading** (measured 2026-10-26, one kill,
  `G1_2`, the log read read-only against the person's clock times). The
  person put the kill at 11:28:17 local time (10:28:17Z). Her last line
  was "Swell!" at 11:28:04, a combat bark like the ones before it. In the
  30 s after the kill the log held one line, a window focus loss at
  11:28:18, a kind already seen twice during her fight; nothing of a kind
  or text not seen earlier in the fight came before the Grelwood's area
  lines at 11:29:19. The controls in the same reading were present: the
  `G1_2` entry at 11:21:02 (person's time 11:21:03), her first line at
  11:27:04 (person's 11:27:07) and the `G1_4` entry at 11:29:20 (person's
  11:29:20). The `+10% to Cold Resistance` reward came at 11:30:53, after
  the item was used, 169 s after her last line, and Una's "What happened
  here...?" at 11:33:19. So her defeat is not observed in the log, with a
  control, from one kill: one controlled negative, not proof that no
  build or setting ever logs it. Her death cry has no text (poe2db's
  `CarrionCroneDeath`, § "Seeing a voiced line from outside the game"),
  which fits a log that carries a voiced line through its words.
- **Live 6, a second controlled kill** (measured 2026-10-26, run as session
  7, § "Live 6: three instruments, one kill"). Her four lines were logged;
  her death, about 15:10:01.5Z by the boss bar and the instruments, was
  again not: the log's only lines between her last line and the reward
  were a level-up of the player's character at her death and a
  shader-compiler line. The `+10% to Cold Resistance` reward came about
  44 s after her death and before the `G1_4` area lines. The person used
  the item before the procedure's 60 s were up, since they had no timer
  (the owner, 2026-10-26), so the reward again came at the use, as in
  Live 1 and Live 4. From outside the game her death was seen: her name left the
  boss bar at the kill, the person heard a cry in the game-only recording,
  and two audio records were read from disk then and at no other time in
  the session. The tracker still has the player tick the defeat step.
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

## Seeing a voiced line from outside the game

Beira of the Rotten Pack's defeat is a voiced death cry with no words, and
`Client.txt` does not carry it (Live 4, § "Rewards after a boss kill"). This
section records what was read about seeing such a line from outside the
game, without touching the game's process: where the client keeps its
audio, how it loads it, and what Live 6's three outside instruments showed.

### The line the log does not carry

- **Her death cry has no text** (static reading of poe2db, fetched
  2026-10-26 by a side session,
  https://poe2db.tw/us/Beira_of_the_Rotten_Pack). Her NPC Text Audio table
  has 22 entries whose keys start with `CarrionCrone`. One of them,
  `CarrionCroneDeath`, has only its own key in the text field, and its
  audio path ends in
  `monsters/part1/act1/carrioncrone/carrioncronedeath.ogg`.
- **Death cries with words are logged** (measured 2026-10-26, the owner's
  `Client.txt`). Lythara's "Argh! You defy the very Spirit itself!",
  "Argh... was I...?" and "Argh.. forgive me.." are in the log as NPC
  lines like any other. `CarrionCrone` appears in the log only in the
  patcher's lines (`Queue file to download:
  FMOD/Desktop/CarrionCrone.bank`). The reading is that the log carries a
  voiced line through its text, so a line with no text leaves nothing to
  carry; that this is the mechanism is not established.
- **The client's settings that touch dialogue** (static reading of the
  owner's `poe2_production_Config.ini`, 2026-10-26).
  `enable_subtitles=true` and `output_all_dialogue_to_chat=true`;
  `mute_in_background=true`; `dialogue_sound_volume2=100`, with the master
  volume at 48 and music and effects at 41 each;
  `borderless_windowed_fullscreen=true` and `fullscreen=false`, at
  2560×1440; `chat_box_width=1040`, `chat_box_height=640` and
  `show_chat_timestamps=true`. Whether `output_all_dialogue_to_chat` is
  what puts NPC lines into the log is not established, and so is whether
  `mute_in_background` silences the game while it is not focused, which
  its name suggests (not measured).

### Where the client keeps its audio (static reading)

- **The GGPK's layout** (static reading, 2026-10-26). A side session's
  script walked the directory records of `Content.ggpk` read-only, with
  the game closed, reading record headers only; the planning run re-read
  its output, which stays outside the repository.
  - `Content.ggpk` is 154,759,947,271 bytes, GGPK version 3, whose names
    are UTF-16.
  - `/Bundles2` holds 67,533 files (131.2 GB), among them `_.index.bin`
    (115,154,591 bytes) and `_.index.low.bin` and `_.index.high.bin`
    (74 bytes each).
  - `/FMOD/Desktop/` holds 2,937 FMOD banks (15.5 GB), stored as plain
    GGPK files, not inside bundles: `CarrionCrone.bank` is 15,376,384
    bytes and `Una.bank` 680,896 bytes, beside `Master Bank.bank` and
    `Master Bank.strings.bank`.
  - Dialogue audio is in bundles under
    `/Bundles2/Streaming/Folders/audio/dialogue/`. Its `4.0/` folder
    holds 965 bundles named only by category (`npc.*`, `characters.*`,
    `monsters.*`, `mtx.*`); its `npc/` folder holds 88 bundles named by
    speaker, none of them for Una. There is also
    `/Bundles2/Folders/audio/dialogue/4.0/npc.bundle.bin`.
  - No bundle's name names the monster: which bundle holds
    `carrioncrone/*.ogg` is written only in the index.
  - A bundle is split into 256 KiB blocks, with a table of the blocks'
    sizes in its header. Offsets move with each patch.
  - The install folder has no `Bundles2` folder of loose files and no
    Oodle DLL; `fmod.dll` and `fmodstudio.dll` sit beside the exe.
- **What the client imports for audio** (static reading of
  `PathOfExile.exe` on disk, 2026-10-26: its import tables and a search of
  its strings, names only).
  - `fmod.dll` and `fmodstudio.dll` are delay-loaded.
  - For banks it links `Studio::System::loadBankCustom` and no other
    loader. `loadBankMemory` and `loadBankFile` are not in the exe as
    ASCII strings at all, so neither is looked up by a stored name; a name
    built at run time would not show in this search.
  - For core sounds it links `System::createStream`. `createSound`,
    `setFileSystem` and `loadSampleData` are neither imported nor in its
    strings, with the same caveat.
  - It also links `EventDescription::isStream` and `getSoundInfo`.
  - From kernel32 it imports `ReadFile`, `GetOverlappedResult(Ex)`,
    `CreateFileMappingW` and `MapViewOfFile(Ex)`, so the imports alone do
    not say whether `Content.ggpk` is read or mapped (the next subsection
    reads that it is mapped).
  - The exe holds the string `Content.ggpk` 4 times in ASCII and 14 times
    in UTF-16, and holds `NPCTextAudio`.

  The reading drawn from the imports, not established: every byte of a
  bank reaches FMOD through the client's own read callback, so a bank's
  reads are the game process's reads of `Content.ggpk`, and dialogue
  reaches FMOD through `createStream`. FMOD's documented behaviour,
  recalled and not fetched in that run, is that a bank's metadata loads
  when the bank loads, its sample data on first use unless loaded
  explicitly, and a streamed sound pulls its data as it plays.

### How the client loads its audio (static reading)

Read on 2026-10-26 in the Ghidra project of a copy of the client
(`PathOfExile.exe`, 78,335,832 bytes, file date 2026-10-19), through the
ghidra MCP's read tools; nothing here was measured. The project is an
import without full analysis, so its cross references were empty for every
import below. The call sites were found instead by a stdlib byte scan of the
exe for indirect calls through each import's slot, kept outside the
repository, and each site's function was taken from the exe's unwind table
(`.pdata`) and decompiled through the MCP. The RVAs below belong to that
build and only help to find the code again; they are a research finding,
never an interface (AGENTS.md § "Never Call an Address You Resolved by
Hand").

- **Q1, banks (`Studio::System::loadBankCustom`).** The exe calls it from
  one place (the function at RVA `0x11CC3C0`). That function builds the
  bank's path as a UTF-16 string, `FMOD/Desktop/` followed by the bank's
  name, and gives it to FMOD as the bank info's user data, with its length
  in bytes. It sets the open, close, read and seek callbacks and no
  asynchronous read callback. The open callback asks the client's own file
  layer for the file's size by that path and hands FMOD a handle that is
  only an entry in a table of the client's (the file, its size and a
  position); it opens no Windows file. The read callback asks the same
  file layer for a byte range at the handle's position. That layer looks
  the path up in the bundle index first; a bank is stored as a plain GGPK
  file under `/FMOD/Desktop/`, not in a bundle (the GGPK walk of
  2026-10-26), so it takes the plain-file route, where the bytes are copied
  out of the file's data inside the client's memory map of `Content.ggpk`
  (Q3). The read callback therefore ends in a memory copy from a mapped
  view, not in `ReadFile`, synchronous or overlapped.
- **Q2, dialogue (`System::createStream`).** The exe calls it from one
  place (the function at RVA `0x11CC100`), an event callback that handles
  the two programmer-sound callback types, create and destroy, going by
  FMOD's documented type values. The event instance's user data carries a
  name. The callback looks that name up in the audio table (`getSoundInfo`),
  takes the sound's mode from the answer, adds the create-stream flag, and
  creates the stream from the name with extended info in which only the
  four file callbacks are set, plus the initial seek position in
  milliseconds when the user data holds a positive start time. The open,
  close and seek callbacks are the bank's own; the read callback is a
  second one that reports end of file on a short read. The length stays
  zero and the client adds neither the open-from-memory nor the
  memory-point flag. A flag in the user data makes it pass no extended
  info at all, which leaves the file to FMOD's own file system. So a
  voiced line is pulled as it plays, through the client's callbacks; it is
  not handed to FMOD whole, in memory. Each of those reads goes through the
  same file layer as a bank's. A file inside a bundle is served by taking
  the 256 KiB block or blocks its range covers from the bundle's bytes in
  the `Content.ggpk` map, decompressing them into a temporary buffer and
  copying the range out. Whether the mode the audio table returns carries
  a memory flag of its own is not established: it is a value the game
  holds at run time.
- **Q3, `CreateFileMappingW` on `Content.ggpk`.** Yes. The object the file
  layer asks for GGPK records opens its file with `CreateFileW` and, when
  the file is not empty, creates one mapping over the whole of it and maps
  views in windows aligned to the system's allocation granularity. The one
  construction followed asks for read and write access, shares read only
  and opens the file if it exists or creates it if not. That this file is
  `Content.ggpk` comes from its context, not from tracing the path
  argument: the global the file layer reads is set by that construction,
  and the client's code that reports and truncates `Content.ggpk` by name
  tears down an object of the same kind first. Every file, a bank or a
  bundle, is then reached through a pointer to its `FILE` record's data in
  that mapping. Without a GGPK object (loose files) each file is mapped
  too: opened read-only with a sequential-scan hint, then mapped whole.
  No `ReadFile` call was on the paths followed here; the exe's nine
  `ReadFile` call sites were not traced.
- **Q4, decompressed bundle blocks.** Not kept on this path: each read
  decompresses the blocks it needs into a temporary buffer and frees it
  after the copy, so a stream's next read of the same block decompresses
  it again. Whether the decompressor keeps a cache of its own was not read
  (not established).

What this predicts for Live 6's file trace (a prediction, not established
until measured): a bank load or a voiced line's playback does not issue a
read call on `Content.ggpk`. It touches pages of the mapping, and a page
reaches the disk only when it is not already in memory, as a paging read
after a hard fault; a page still in the system's cache is a soft fault and
leaves no file I/O. A file trace that sees nothing at the kill is what this
reading predicts, and would not show that nothing played. Reading
`Content.ggpk` while the client runs: the client's handle asks for write
access and shares read only, so Windows lets another process open the file
only for reading and only with a share mode that allows writing; that was
not tried.

### Live 6: three instruments, one kill

Measured 2026-10-26 with Live procedure 6 and one Beira kill, by a level-2
Druid in Clearfell (`G1_2`), the three instruments of
[`outside-instruments.md`](outside-instruments.md) running outside the game
and nothing installed into it. The first attempt was blind at its start
(the instruments' spawn did not run their command) and spent no kill; the
procedure was run again as session 7, after a Restart, and every verdict
below is that session's (capture
`.claude/workorders/quest-tracker-beira-defeat-live-7.md`, its analysis
kept outside the repository). Times are UTC; the log's local time was
UTC+1. Her lines were four: her opening line at 15:09:38.8, "Invader!",
"Be still!" and "Feed!" by 15:09:50. The kill marker M, the person's press,
was at 15:10:05.535, "less than 5 seconds" after her death by their
account, which they confirmed. The boss bar's last frame with her name was
4.0 s before M, and the log's level-up line (16:10:01 local) came at about
the same moment, so her death was close to 15:10:01.5, about 4 s before M.
The reward line came at 15:10:45.655 and Una's "What happened here...?" at
15:11:25.749 in `G1_4`.

| Check | Verdict | Observed |
|---|---|---|
| `control` | pass | one `LOG FILE OPENING` (15:01:40), 672 lines from the lease's pid, `G1_town` at 15:02:03.6 |
| `marker` | pass | 2 presses, the test press at 15:04:05.3, M at 15:10:05.535, 26.69 s after her first line and 40.1 s before the reward line; the person confirmed it was at her death |
| `ggpk-stable` | pass | the GGPK tree identical before and after the session |
| `i1-rehearsal` | pass | both probe reads seen with the probe's pid, the cached one included |
| `i1-rehearsal-mapped` | pass | the probe's touch of a mapped page seen as a paging read 0.001 s later |
| `i2-rehearsal` | pass | the selftest tone 95.9 dB above the silent one, onset +105 ms |
| `i3-rehearsal` | pass | 5 frames, none black; both probe lines OCR'd at 1.00 |
| `i2-town` | pass | 2 of Renly's lines, burst onsets +0.20 s and +0.12 s |
| `i2-isolation` | pass | the test press's beep 8.3 dB above its neighbours in the game-only recording (below the 15 dB that would mean other processes leaked in) |
| `i3-town` | pass | both of Renly's lines OCR'd at 1.00 in 8 frames each; no black frames |
| `i1-control` | pass | a read of R at 4 of her 4 lines; R was `CarrionCrone.bank` and one dialogue bundle, `monsters.D1` (no decoded index) |
| `i1-control-una` | pass | Una's line: a read of `/Bundles2/Streaming/Folders/audio/dialogue/4.0/npc.12.bundle.bin` 0.02 s after it |
| `i1-read-timing` | pass | bank: playback; dialogue: playback; town lines read: 2 of 2 |
| `i1-target` | not-observed | 3 reads of R within [M − 3, M + 2] with none of her lines near, but a base rate of 0.50 over the fight's 2 line-free windows, which is not below 0.5; cache cold (boot after the last game start) |
| `i2-control` | pass | a burst onset at 3 of her 4 lines (0.75), base rate 0.00 over 2 windows |
| `i2-control-una` | pass | Una's line: onset +0.15 s |
| `i2-target` | pass | one onset at M + 1.31 s, base rate 0.00 over 2 windows |
| `i2-listened` | pass | the person heard her voice in the bark clip and a cry in the kill clip |
| `i2-template-control` | not-run | no reference audio in `ref\` (no decoder in the session's folder) |
| `i2-template` | not-run | no reference audio in `ref\` |
| `i3-control` | pass | 4 of her 4 lines OCR'd in the chat band; no black frames |
| `i3-control-una` | pass | Una's line at 1.00 in 8 frames |
| `i3-bar-control` | pass | her name in the boss bar in 12 of 23 top-band frames between her first and last lines (0.52) |
| `i3-text-target` | pass | 5 bottom-band lines new within [M − 3, M + 4], at −2.51, −2.01, −0.50 and twice +3.50 s |
| `i3-bar-target` | pass | her name in 12 of 20 top-band frames in the 10 s before M and in none of 18 frames from M + 1 s to M + 10 s, none black; the last frame with it 4.02 s before M |
| `reward-split` | fail | the reward line 40.1 s after M, not the 55 s the procedure's 60 s wait would give; the person used the item early (no timer), so the fail records a short wait, not a reward granted at the kill |

**The file-read trace (I1).** Its controls passed: the rehearsal saw a
cached read call and a paging read of a mapped page, the cache was cold
(the boot came after the last game start), and each of her lines and Una's
line had a read of the matching audio within 0.5 s before to 1.5 s after
it (Una's 0.02 s after).
Every one of the game's 62,562 reads of `Content.ggpk` in the session was a
paging read, with no read call among them, which is what the static
reading above predicts for a memory-mapped file. Read timing is
`playback` for both parts of R: `CarrionCrone.bank` was not read at the
area's load or at her first line but from her second line on, 136 paging
reads in bursts every 1 to 4 s through the fight, the last
at 15:10:02.87, about 1.3 s after her death, and none after it. The
`monsters.D1` dialogue bundle was read at her first three lines. The
target is `not-observed`, but not for the reason its fixed wording gives:
reads of R were seen inside the kill window (three of the bank, at
15:10:02.87), and the check refused them because the bank is read as often
during the fight as at the kill, so they mark the fight going on rather
than its end. Outside the check, two records were read at her death and at
no other time in the whole session: the dialogue bundle
`/Bundles2/Streaming/Folders/audio/dialogue/4.0/monsters.F4.bundle.bin`
(its blocks 0 and 1, at 15:10:01.95) and `/FMOD/Desktop/BossKill.bank`
(15:10:02.00), both about 0.5 s after the bar's last frame and about 3.5 s
before M, so outside the check's [M − 3, M + 2]. A characters dialogue
bundle (`characters.7E`) was read at 15:10:02.93, 0.1 s before the
subtitle of the player character's own remark appeared. Whether
`monsters.F4` holds `carrioncronedeath.ogg` is not established: no decoded
index was made, since the session's folder held no decoder. That
`BossKill.bank` serves every boss kill is read from its name only. Both
were first reads after a cold boot; a second kill in the same boot may find
the pages in the cache and leave no read (the static reading above), which
this one kill does not test. So the negative does not show that the file
route cannot see her death, and the extra reads do not show that it can
see it on a warm cache.

**The loopback recorder (I2).** Its controls passed: the selftest tone,
the isolation check on the test press, both of Renly's town lines, 3 of
her 4 lines with a base rate of 0.00, and Una's line. The target passed by
its rule, with one burst onset at M + 1.31 s, and the person heard a cry in
the kill clip, lasting by their account to about 2 s before the press and
followed by the level-up sound and the character's remark. The onset the
rule found is about 5 s after the bar's last frame and after the cry the
person describes, so it is more likely a later sound (the dropped items or
the remark) than the cry's start; that the burst rule picks the cry out of
the fight's sound is not established. Both base rates come from only two
line-free windows. The template checks did not run, for want of reference
audio. So the audio route's evidence is the person's ear and a burst near,
not at, the death.

**Screen capture and OCR (I3).** Its controls passed: the rehearsal, the
town lines, all four of her lines and Una's line read from the chat band,
and her name in the boss bar in 12 of 23 frames during the fight, just
above the 50% the control asks for. No frame of 2,110 was black, so a GDI
capture of the borderless window works on this client. Her name read in
12 of the 20 frames in the 10 s before M and in none of the 18 after
M + 1 s; the last frame with it was 4.02 s before M, within about 0.5 s of
her death as the level-up line and the two reads above place it. The text
target passed, but none of its 5 new lines is hers (her cry has no text):
they are the player character's own remark as a subtitle, prefixed with the
character's name, the name of an item she dropped, the skill bar's key
hints and two labels of dropped items. The remark is not in `Client.txt`
(the log holds only the level-up line and a shader-compiler line between her
"Feed!" and the reward line); whether the character makes it at every boss
kill is not established.

**The reward.** It came 40.1 s after M, about 44 s after her death. The
person used the item before the 60 s wait was over, having no timer (the
owner, 2026-10-26), so the wait was shorter than the procedure asked and
the reward followed the use, as Live 4's did.

### Product route

Recommended route: boss bar

Taken from the decision table of this workorder: the boss bar is viable
(`i3-bar-control` and `i3-bar-target` passed) and comes first. The audio
template is viable by the same table too (`i2-control`, `i2-target` and
`i2-listened` passed), with the caveat above that its onset came after the
cry. File reads are not viable on this evidence (`i1-target` was
`not-observed`), which does not show that they cannot be.

- **The boss bar** needs no admin and no game asset, and it covers every
  boss with a bar. A bar also goes away when the player leaves its range
  or dies, so a detector has to pair the bar's going with her lines before
  it and with no `has been slain` line. Her name read in only about half
  the frames during the fight, so the detector needs a run of frames, not
  one. One kill, one character, English client, borderless windowed at
  2560×1440.
  - **A covered bar** (measured in Live 8, on The Bloated Miller alive;
    § "What the screen shows of each source"). This session had no panel
    open during the fight. In Live 8 the bar's name still read with the
    inventory, character panel, Escape menu or map overlay open, and did
    not with the passive tree or the world map open, whose own text takes
    the bar's band. A covered bar that hides the kill
    only costs a tick the reward line gives anyway; a panel that hides
    the bar would read as her death. So the detector should class each
    frame as bar present, bar absent, or bar not visible, and tick only on
    present followed by a run of visible-and-absent frames with no
    not-visible gap between them. The owner's direction (2026-10-26) is to
    use the bar for bosses whose death leaves no log line. Live 8 took
    over the panel measurement the build was to start with, on one boss
    with the player keeping its distance, and the tracker overlay must
    stay off the bar's band.
- **The audio template** needs no admin, but a template is an extracted
  asset that cannot ship, so the product would have to rebuild it, boss by
  boss, from the player's own install.
- **File reads** need an admin trace for the overlay's whole lifetime and a
  map rebuilt each patch. The game reads `Content.ggpk` through a memory
  map (all 62,562 of its reads were paging reads), so the trace sees a read
  only when a page comes from disk. A read at a cold-cache kill, like the
  `BossKill.bank` and `monsters.F4` reads above, does not show that a
  shipped detector would see a second kill, or a kill whose pages are still
  cached.

The value is small: the reward line already finishes the quest from
either step, usually within a minute (Live 4: tick at 10:30:13Z, done at
10:30:53Z; this session: about 44 s after her death). The gain is one tick
for each of three bosses (Beira, Blackjaw, Lythara), measured on one kill,
and building it is the owner's call. A fourth route the owner asked about,
reading the game's window through Windows' screen reader (Narrator or UI
Automation), was not one of the three planned instruments and was not
explored; whether the client exposes any text that way is not measured.

## On-screen text and the dialogue setting

The `quest-tracker-screen-text` workorder asks what the screen shows of
three sources an on-screen detector could read: a boss's bar (under each
game panel, and after the player walks away), the character's remark at a
kill, and an NPC's quest dialogue window (Renly's talk after The Bloated
Miller). It also asks whether the client option
`output_all_dialogue_to_chat` decides whether NPC lines reach
`Client.txt`. Live 8, one session on a fresh character, measures both.
This section starts with what was read before that session.

### The dialogue setting (static reading)

Read on 2026-10-26 while planning the workorder, from the same copy of the
client as § "How the client loads its audio (static reading)", through the
ghidra MCP's read tools and a stdlib byte scan of the exe kept outside the
repository; the game was not running and nothing here was measured. The
build's addresses are left out on purpose: they only help to find the code
again in that one build (AGENTS.md § "Never Call an Address You Resolved by
Hand").

- **The config file** (static reading of the owner's
  `poe2_production_Config.ini`, read-only). It is UTF-8 with a byte-order
  mark and LF line endings. `output_all_dialogue_to_chat=true` sits in the
  `[UI]` section and `enable_subtitles=true` in `[GENERAL]`. The panel key
  bindings in `[ACTION_KEYS]` are stored as Windows virtual-key codes; on
  that file they are I for the inventory, C for the character panel, P for
  the passive tree, U for the world map and Tab for the map overlay, and
  Options is on O.
- **The option is looked up each time it is used.** The client carries
  both keys as UTF-16 strings. One small accessor asks the client's
  in-memory options for section `UI`, key `output_all_dialogue_to_chat`,
  every time it is called, and nothing keeps a copy taken at start-up. A
  matching setter has a single caller. The prediction that follows, not
  measured, is that changing the option in the Options panel applies from
  the next line on, with no restart.
- **What it gates.** Four functions call the accessor directly. In two of
  them the client is about to pass a voiced line, a speaker and its text,
  on to the chat, and with the option off it returns without passing it.
  One of those two posts the line as a chat message on a fixed channel,
  and adding a chat message notifies whatever listens to the chat.
- **Not established.** Which part of the client writes NPC lines into
  `Client.txt`, and so whether the log is one of the chat's listeners and
  loses those lines when the option is off, is not established by this
  reading. Live 8 measures it, with the option on, off and on again, on
  Renly's vendor greetings in Clearfell Encampment.
- **The instrument.** The MCP's string search and its reference table came
  back empty for the option's key, but they were just as empty for
  `LOG FILE OPENING`, a string the client certainly prints, so this import's
  tables are blind here and their silence says nothing either way. The
  callers were found instead by scanning the code section for
  RIP-relative address loads of the key and direct relative calls to the
  accessor and setter, with each caller's bounds taken from the unwind
  table (`.pdata`).

### Live 8: on-screen text on a fresh character

Measured 2026-10-26 with the workorder's Live procedure 1 (capture
`.claude/workorders/quest-tracker-screen-text-live-1.md`; the live folder,
its frames and `analysis.json` kept outside the repository), on a fresh
Warrior made for the session, English client, borderless windowed at
2560×1440. Nothing was installed into the game: the screen capture (2 Hz,
the top-centre band and the whole window as the bottom band, `run start
--bottom 0,0,1,1`), the marker and the log reader ran outside it. Times
are UTC; the log's local time was UTC+1. The character entered The
Riverbank (`G1_1`) at 20:06:37.7.

- **The bar runs** were on The Bloated Miller, alive. The first `boss` run
  (from 20:08:15) is ignored by the analysis: its 9 presses do not pair,
  because M opened the in-game shop, the panels came in another order and
  the world map was opened after the walk away. The second run (20:14:24
  to the walk-away press at 20:17:54.2) took the panels in the asked
  order, inventory, character panel, passive tree, world map, Escape menu
  and map overlay, each open for 4.7 to 7.1 s. The character did not die
  and the Miller lived.
- **The remark run** was the Miller's kill: the kill press at
  20:19:45.913, and one press after each of the two remarks the person
  heard (by their account a generic one after a strong foe, then one
  about the Miller). The character went from level 1 to 2 at the kill. No
  Beira repeat was needed.
- **The dialogue run** was Renly's talk in Clearfell Encampment
  (`G1_town`, entered at 20:23:01.9), from the open press at 20:23:26.327
  to the close press at 20:23:59.655. The person chose a reward and
  accepted the next quest, and did not report a topic's text, so
  `dlg-option` did not run.
- **The setting runs** were 3 of Renly's vendor greetings with the option
  on (20:27), the option turned off in Options, where it is labelled
  "Output Dialogue To Chat" in the UI tab (20:28:50 to 20:31), 3 greetings
  with it off (20:31 to 20:32), 2 with it on again (20:34), then an exit,
  the game started again with the file reading `false`, and 2 greetings in
  the relaunched client (20:36). The capture stopped at the first exit (the
  window was gone), so the relaunch has log lines and presses but no
  frames.

| Check | Verdict | Observed |
|---|---|---|
| `control` | pass | two `LOG FILE OPENING`s, the second the relaunch; 1,148 lines from the first client's pid; area entries `G1_1` at 20:06:37.7 and `G1_town` at 20:23:01.9 |
| `marker` | pass | 39 presses, the test press at 20:08:03.4; the person confirmed the game did nothing on it |
| `i3-rehearsal` | pass | 5 frames, none black; both probe lines read at 1.00 |
| `i3-town` | pass | each of Renly's 3 `town-on` greetings read at 1.00 in 8 frames; no black frame |
| `cfg-on-log` | pass | 3 of 3 presses with a Renly line logged 1.2 to 2.3 s before the press |
| `cfg-setting-found` | pass | found, labelled "Output Dialogue To Chat", in the UI tab |
| `cfg-file-written` | pass | the `after-toggle-off` record reads `false`; the client wrote the file at 20:30:55.3, while it ran |
| `cfg-off-shown` | pass | 3 of 3 `town-off` presses with a new Renly-prefixed line on screen, first frames 0.7 to 3.2 s before the press |
| `cfg-off-log` | pass | 3 of 3 shown greetings logged with Renly as speaker, 1.0 to 1.4 s before the press |
| `cfg-on-again-log` | pass | 2 of 2 presses with a Renly line logged about 1 s before the press |
| `cfg-off-relaunch-log` | pass | the relaunched client (its own `LOG FILE OPENING`, 611 lines, `G1_town` at 20:36:11.4) started with the file reading `false` (`after-exit-off`); 2 of 2 presses with a Renly line from it about 1 s before the press |
| `dlg-window` | pass | 16 new lines of 3 or more words in the 33.3 s, none of them speaker-prefixed, and Renly's name at 1.00; union box 0.030,0.311 to 0.975,0.995 of the client rect, which also takes in the lower-left line list and the skill bar's key hints |
| `dlg-option` | not-run (instrument: no `--dialogue-option`) | the topic's text was not reported |
| `dlg-subtitle` | fail | no new Renly-prefixed line in 71 frames from the open to 2 s after the close |
| `dlg-log` | not-observed | no Renly line logged from the open to 2 s after the close, greeting or not, and none of the 16 window lines repeated by a logged line |
| `bar-control` | pass | the Miller's name in 29 of 29 top-band frames before the first panel; the band's median luma 17.1 mean, 21.6 standard deviation |
| `bar-inventory` | pass | 12 of 12 frames; luma 27.3 and 27.4; 15 other lines in the band |
| `bar-character` | pass | 10 of 10 frames; luma 15.6 and 22.5; 7 other lines |
| `bar-passive-tree` | fail | 0 of 7 frames; luma 26.3 and 39.2; 7 other lines, the panel's title |
| `bar-world-map` | fail | 1 of 11 frames; luma 28.5 and 34.6; 12 other lines, the map's heading |
| `bar-escape-menu` | pass | 8 of 8 frames; luma 9.6 and 14.7, the screen darkened; no other line |
| `bar-map-overlay` | pass | 9 of 9 frames; luma 16.9 and 21.1; no other line |
| `bar-away` | pass | the name in none of 10 top-band frames from 1 s to 6 s after the walk-away press |
| `remark-shown` | pass | both presses after the kill had a new line prefixed with the character's name, first frames 5.4 s and 4.8 s before them (see below for which lines) |
| `remark-kill` | pass | a new prefixed line from 0.55 s before the kill press, in 34 frames (see below) |
| `remark-log` | not-observed | neither of the 2 lines the remark checks found was logged |

**Which lines the remark checks found.** The two checks passed by their
rule, but not on the kill remarks. Reading the session's OCR cache after
the analysis, for times and positions only, the lines they credited were
one of the character's combat shouts, on screen from 20:19:40.4 (5.5 s
before the kill press), and a read of the character's name and colon
alone, the rest of its line split off by OCR. The two remarks the person
heard showed in the same place, each prefixed with the character's name:
the generic one from 20:19:46.35 (0.44 s after the kill press) and the
one about the Miller from 20:19:48.86 (2.95 s after it). So the remark is shown at a
kill, and the checks' pass does not show it; the character's combat shouts
carry the same prefix. The Miller's own two combat lines showed in the same
list with his name as prefix, and both are in `Client.txt`. The
character's lines are not: `Client.txt` from 21:19:35 to 21:19:59 local
holds one of the Miller's lines, the level-up line and chat, and nothing
else.

**The bar at the kill.** The Miller's name was last read in the
whole-window band at 20:19:44.85, 1.06 s before the kill press, and the
level-up message was on screen from 20:19:45.36. The top band's frames
around the kill were not read, since no check needs them.

**The dialogue window** (positions read from the OCR cache, as fractions
of the client rect). Its first page was already on screen in the earliest
frame read, 5 s before the open press, so the press came after the window
opened and that page counts as old to `dlg-window`; the later pages
supplied its 16 new lines. The window is drawn in the centre: Renly's name
as a heading at about y 0.27 and again at y 0.46, the text in a block at
x 0.42 to 0.66, y 0.51 to 0.61, about 0.02 of the height per line, and a
"Continue" button below it at y 0.62. The reward choice ("Select One
Reward", at y 0.40) sat above the text; while the inventory was open to
take the reward the text block moved left, to x 0.25 to 0.49. A "QUEST
COMPLETE" banner showed at y 0.78, then the next quest's banner in the same
place, and at the end the topic list ("The Miller", "The Devourer", "Buy or
Sell items", "Goodbye") in two columns at x 0.40 to 0.69, y 0.52 to 0.62.
No line of the window carries a speaker prefix, and the character's own
remark after the talk, which the person also heard,
came as a prefixed line in the lower-left list at 20:23:56.9, as the window
closed.

**The dialogue setting.** Renly's greetings were logged in every state:
with the option on (3 of 3), turned off in the running game (3 of 3), on
again (2 of 2), and in a client started with the file reading `false` (2
of 2), each 1 to 2.3 s before its press. With the option off they also
showed on screen as before, in the same place (the lower-left list, at x
0.03, y 0.72). The client wrote the config file while it ran, at 20:30:55.3
after the toggle off and at 20:34:12.5 after the toggle on (3.3 s before
the next greeting's press), and again at the exit, at 20:35:14.9, with the
same contents as after the toggle off. Whether the toggle applies from the
next line on cannot be told from greetings that are logged in every state.
Which lines the option does change is not established: only vendor
greetings were measured, and the static reading above finds voiced lines
the option keeps from the chat, not which lines they are.

### Dialogue setting route

Dialogue setting: no-effect

Taken from the workorder's route table: `cfg-off-log` and
`cfg-off-relaunch-log` both passed. Renly's vendor greetings reached
`Client.txt` with `output_all_dialogue_to_chat` off, both after the in-game
toggle and in a client started with the file reading `false`, so the
quest tracker does not need the option, and the README names no
prerequisite for it. This covers vendor greetings, the only kind of NPC
line measured. A quest NPC's talk in a dialogue window was not logged with
the option on either (`dlg-log`, as in Lives 1 and 3), so it tells nothing
about the option.

### What the screen shows of each source

- **The boss bar** (Live 8 on the Miller alive, Live 6 on Beira's fight).
  The name sits in the top-centre band, at x 0.456 to 0.544, y 0.047 to
  0.060 of the client rect at 2560×1440. With no panel open it read in 29
  of 29 frames on the Miller, while the player kept their distance, and in
  about half the frames during Beira's fight. Under each panel:
  - *present* (the name still reads): the inventory, the character panel,
    the Escape menu (the whole screen darkened, the band's luma mean 9.6)
    and the map overlay;
  - *not visible*: the passive tree, a full-screen panel whose title takes
    the band (0 of 7), and the world map, whose own heading is drawn over
    the band (1 of 11).

  Present, absent and not visible differ in the band itself, in this one
  session: with the bar up its median luma was 17.1 mean and 21.6
  standard deviation; with the bar gone after the walk away about 14.6 and
  7.5 per frame (read from `frames.jsonl` after the analysis), with no
  text; under the two covering panels 26 to 29 and 35 to 39, with the
  panel's own text in the band. The bar went when the player walked away
  while the Miller lived (`bar-away`), so its going is not a kill by
  itself. At the Miller's kill the name's last frame was 1.06 s before the
  kill press; at Beira's it was 4.0 s before a press made about 4 s after
  her death (Live 6).
- **Voiced lines and the remark** (Lives 6 and 8). Every voiced line read
  on screen in Live 8, the Miller's combat lines, Renly's greetings with
  the option on and off, and the character's shouts and remarks, showed as
  one line `<speaker>: <text>` in a list at the lower left: from x 0.03,
  the newest line at about y 0.72, older lines moving up about 0.02 per
  line. The level-up message showed in the same list. Whether the list is
  the chat's recent-message display or the subtitle display is not
  established. The character's lines carry the character's name as their
  prefix and are not in `Client.txt`; the NPCs' and the boss's lines are.
  The kill remarks came 0.4 s and 3.0 s after the kill press (Live 8) and
  about 1.5 s after Beira's death (Live 6). A kill remark is told from a
  combat shout by its text, such as the class's `MillerKillComment` in
  poe2db's table (§ "Character remarks"), not by its prefix or its time.
- **The dialogue window** (Live 8, Renly). It draws in the centre of the
  screen, its text block at x 0.42 to 0.66, y 0.51 to 0.61, shifted left
  while the inventory is open, with Renly's name as a heading above it.
  OCR reads it cleanly: Renly's name at 1.00 and 16 lines of 3 or more
  words, each in 2 or more frames. None of its lines carries a speaker
  prefix, none is repeated in the lower-left list as Renly's line
  (`dlg-subtitle`), and none reaches `Client.txt` (`dlg-log`). A detector
  has to read the window itself; the "QUEST COMPLETE" banner and the next
  quest's banner below it, at y 0.78, are shorter targets in the same
  frames.

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
