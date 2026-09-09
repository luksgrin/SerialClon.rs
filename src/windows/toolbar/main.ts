import { mount } from 'svelte';
import '../../app.css';
import { initPrefs } from '$lib/prefs';
import { installHelpBus } from '$lib/help-bus';
import Toolbar from './Toolbar.svelte';

installHelpBus('Toolbar');
initPrefs();
mount(Toolbar, { target: document.body });
