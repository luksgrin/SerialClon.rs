import { mount } from 'svelte';
import '../../app.css';
import { initPrefs } from '$lib/prefs';
import { installHelpBus } from '$lib/help-bus';
import { installEditMenu } from '$lib/edit-menu';
import Find from './Find.svelte';

installHelpBus('Find');
installEditMenu();
initPrefs();
mount(Find, { target: document.body });
