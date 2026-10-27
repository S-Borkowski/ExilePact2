Story and evidence behind `AGENTS.md` § ["Check a Permission Where It Is Used, Not Where It Is Convenient"](../../AGENTS.md#check-a-permission-where-it-is-used-not-where-it-is-convenient).

> From the sister project [hero-siege-offline-toolkit](https://github.com/falorfrozen-cmd/hero-siege-offline-toolkit)
> (Hero Siege, a GameMaker game): the names below are that project's, kept as evidence.

## Map reveal's pack pass

Map reveal's pack pass learned this the expensive way. Its "lie about
distance" permission was invalidated in `OnFrame` when the zone changed, which
looked correct and passed its tests, but the creators consuming that
permission ran earlier in the same frame — so the first call in a new zone
still got the previous zone's answer, which is exactly the call that leaves a
spawner inert. Two rounds of adding more identity tracking at `OnFrame` could
not have fixed it; only moving the check to the consumer did.
