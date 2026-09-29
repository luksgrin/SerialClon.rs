import { mount } from 'svelte';
import '../../app.css';
import { initPrefs } from '$lib/prefs';
import { installHelpBus } from '$lib/help-bus';
import { installEditMenu } from '$lib/edit-menu';
import VirtualCutter from './VirtualCutter.svelte';

installHelpBus('Virtual Cutter');
installEditMenu();
initPrefs();
mount(VirtualCutter, { target: document.body });
