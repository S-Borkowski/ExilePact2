<script lang="ts">
  // Scaffold placeholder; the plain-ui item renders the View pushed by Rust.
  const title = 'Quest Tracker';
</script>

<main class="placeholder">{title}</main>

<style>
  /* The window is transparent; only the panel draws. */
  :global(html),
  :global(body) {
    margin: 0;
    background: transparent;
  }

  .placeholder {
    padding: 8px 12px;
    font: 14px system-ui, sans-serif;
    color: #e8e2d0;
    background: rgba(16, 14, 12, 0.8);
  }
</style>
