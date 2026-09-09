import { mount } from 'svelte';
import '../../app.css';
import { initPrefs } from '$lib/prefs';
import { installHelpBus } from '$lib/help-bus';
import Gateway from './Gateway.svelte';

installHelpBus('Gateway Cloning');
initPrefs();
mount(Gateway, { target: document.body });
