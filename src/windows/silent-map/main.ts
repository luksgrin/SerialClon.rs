import { mount } from 'svelte';
import '../../app.css';
import { initPrefs } from '$lib/prefs';
import { installHelpBus } from '$lib/help-bus';
import { installEditMenu } from '$lib/edit-menu';
import SilentMap from './SilentMap.svelte';

installHelpBus('Silent Map');
installEditMenu();
initPrefs();
mount(SilentMap, { target: document.body });
