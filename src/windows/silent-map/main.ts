import { mount } from 'svelte';
import '../../app.css';
import { initPrefs } from '$lib/prefs';
import { installHelpBus } from '$lib/help-bus';
import SilentMap from './SilentMap.svelte';

installHelpBus('Silent Map');
initPrefs();
mount(SilentMap, { target: document.body });
