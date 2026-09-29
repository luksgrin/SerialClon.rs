import { mount } from 'svelte';
import '../../app.css';
import { initPrefs } from '$lib/prefs';
import { installHelpBus } from '$lib/help-bus';
import { installEditMenu } from '$lib/edit-menu';
import Prefs from './Prefs.svelte';

installHelpBus('Preferences');
installEditMenu();
initPrefs();
mount(Prefs, { target: document.body });
