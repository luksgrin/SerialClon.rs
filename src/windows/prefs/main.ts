import { mount } from 'svelte';
import '../../app.css';
import { initPrefs } from '$lib/prefs';
import { installHelpBus } from '$lib/help-bus';
import Prefs from './Prefs.svelte';

installHelpBus('Preferences');
initPrefs();
mount(Prefs, { target: document.body });
