<script lang="ts">
  // How to grab an item, in two numbered steps: the game's own Ctrl+Alt+C,
  // then the app's hotkey (from the view) or Ctrl+V in this window. Mode
  // none and mode refused both show it.
  let { hotkey }: { hotkey: string } = $props();
</script>

<ol class="grab-steps">
  <li>In the game, hover the item and press <kbd>Ctrl+Alt+C</kbd>.</li>
  <li>Press <kbd class="hotkey">{hotkey}</kbd>, or press <kbd>Ctrl+V</kbd> in this window.</li>
</ol>

<style>
  .grab-steps {
    margin: 0;
    padding-left: calc(var(--space-unit) * 5);
  }

  li + li {
    margin-top: var(--space-unit);
  }
</style>
