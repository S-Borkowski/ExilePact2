// The Settings window's sections, drawn in this order. A later setting is a
// new entry here (and its component), not a new layout: each section gets
// its heading, its region and the divider before it from the window.

import type { Component } from 'svelte';
import type { CatalogueGroup } from '../ipc';
import type { SettingsSend } from './bridge';
import type { SettingsView } from './view';
import BlockedCurrenciesSection from './BlockedCurrenciesSection.svelte';
import GrabSection from './GrabSection.svelte';
import OverlaySection from './OverlaySection.svelte';
import UpdatesSection from './UpdatesSection.svelte';

export interface SectionProps {
  view: SettingsView;
  send: SettingsSend;
  /** The catalogue the blocked currencies are picked from; empty until it has loaded. */
  catalogue: CatalogueGroup[];
}

export interface SettingsSection {
  /** A stable id: the region's heading id is `section-<id>`. */
  id: string;
  /** The section's heading, a group label. */
  title: string;
  component: Component<SectionProps>;
}

export const SECTIONS: readonly SettingsSection[] = [
  { id: 'grab', title: 'Grab', component: GrabSection },
  { id: 'overlay', title: 'Overlay', component: OverlaySection },
  { id: 'blocked', title: 'Blocked currencies', component: BlockedCurrenciesSection },
  { id: 'updates', title: 'Updates', component: UpdatesSection },
];
