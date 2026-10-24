//! Log follower: polls Client.txt on its own thread, opens it read-only only
//! on a poll that sees new bytes, and sends complete decoded lines with their
//! end offset and a catch-up flag over a channel. It never touches quest
//! state.
//!
//! Scaffold stub; the tailer item fills it in.
