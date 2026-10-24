// The overlay UI's entry point: mounts the root component into #app.
import { mount } from 'svelte';
import App from './App.svelte';

const target = document.getElementById('app');
if (!target) {
  throw new Error('quest-tracker: #app is missing from index.html');
}

export default mount(App, { target });
