import { mount } from 'svelte';
import '../../app.css';
import { initPrefs } from '$lib/prefs';
import { installHelpBus } from '$lib/help-bus';
import VirtualCutter from './VirtualCutter.svelte';

installHelpBus('Virtual Cutter');
initPrefs();
mount(VirtualCutter, { target: document.body });
