# A release list for the update check's tests

`releases.json` is written by hand, never fetched: it has the shape of
GitHub's REST "List releases" answer (`tag_name`, `draft`, `prerelease`,
`html_url`, the fields the update check reads), with entries chosen to take
every path of `qt_app::updates::newest_release`:

- `quest-tracker-v1.10.0` is the newest published Quest Tracker release, and
  its `html_url` points elsewhere: the page the app opens is built from the
  tag, never taken from the answer;
- `quest-tracker-v1.9.3` is older, and lower than 1.10.0 only when the parts
  are compared as numbers, not text;
- a draft (1.12.0) and a prerelease (1.11.0) are newer and skipped;
- another tool's tag, and tags that are not exactly `quest-tracker-v`
  followed by `<major>.<minor>.<patch>` (two or four parts, a suffix, a sign,
  a space, an empty part, letters, another case, no prefix, a number too
  large), are skipped;
- entries with no tag, a tag that is not a string, a `draft` that is missing
  or not a bool, and entries that are not objects are skipped, never an
  error.

The tests are `releases_*` in `crates/qt-app/tests/updates.rs`.
