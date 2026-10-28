// The Settings window's sections, drawn in this order. A later setting is a
// new entry here (and its component), not a new layout: each section gets
// its heading, its region and the divider before it from the window.

import type { Component } from 'svelte';
import type { SettingsSend } from './bridge';
import type { AppSettingsView } from './view';
import UpdatesSection from './UpdatesSection.svelte';

export interface SectionProps {
  view: AppSettingsView;
  send: SettingsSend;
}

export interface SettingsSection {
  /** A stable id: the region's heading id is `section-<id>`. */
  id: string;
  /** The section's heading, a group label. */
  title: string;
  component: Component<SectionProps>;
}

export const SECTIONS: readonly SettingsSection[] = [{ id: 'updates', title: 'Updates', component: UpdatesSection }];
