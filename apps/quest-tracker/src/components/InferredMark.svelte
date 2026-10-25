<script lang="ts">
  // The "probably done" mark: a quest or step the view shows done because of
  // a later event the log recorded, not its own signal (`inferred`). A 10 px
  // dashed ring beside the done mark, never in its place: it differs from the
  // done check in shape, so it reads without colour. The tooltip and the
  // accessible name are the same words. It is not a control: no click, no
  // hover state, no focus stop; in edit mode a click on it is a click on
  // whatever holds it (the row opens its detail), as anywhere else there.
  import { INFERRED_LABEL } from '../labels';
</script>

<span class="inferred-mark" role="img" aria-label={INFERRED_LABEL} title={INFERRED_LABEL}>
  <svg viewBox="0 0 10 10" width="10" height="10" aria-hidden="true" focusable="false">
    <circle cx="5" cy="5" r="4.25" fill="none" stroke="currentColor" stroke-width="1.5" stroke-dasharray="2 1.34" />
  </svg>
</span>

<style>
  .inferred-mark {
    display: inline-flex;
    flex: none;
    align-items: center;
    justify-content: center;
    width: 10px;
    height: 10px;
    color: var(--color-text-muted);
  }

  svg {
    display: block;
  }
</style>
