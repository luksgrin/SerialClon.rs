import { mount } from 'svelte';
import '../../app.css';
import { initPrefs } from '$lib/prefs';
import { installHelpBus } from '$lib/help-bus';
import { installEditMenu } from '$lib/edit-menu';
import Toolbar from './Toolbar.svelte';

installHelpBus('Toolbar');
installEditMenu();
initPrefs();
mount(Toolbar, { target: document.body });
