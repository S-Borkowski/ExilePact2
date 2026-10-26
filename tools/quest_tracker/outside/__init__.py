"""Developer-only instruments that watch the game from outside its process.

These tools exist for one research question: can an event the game does not
write to `Client.txt` (Beira of the Rotten Pack's wordless death cry is the
first case) be seen from outside the game at all? Each instrument observes
something Windows already offers to any program: a file-I/O trace of the
game's reads of `Content.ggpk`, a process-loopback recording of the game's
audio, and screen captures of the chat and boss-bar regions read by OCR
afterwards. Around them sit the live folder, the clock anchors that turn the
log's millisecond counter into UTC, the marker press and the offline analysis.

Nothing here ever opens the game's process, reads or writes its memory,
attaches a debugger or calls any of the game's code. Nothing here ships to
players or runs inside the quest tracker overlay. Every command that writes
output refuses a path inside a git checkout: captures, recordings, traces and
anything decoded from the game's files stay in the live folder,
`%USERPROFILE%\\quest-tracker-live-5\\beira-defeat\\` by default.

`common` holds what every instrument shares; `session` prepares the live
folder, records clock anchors and asks the instruments to stop.
"""
