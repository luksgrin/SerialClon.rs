import { mount } from 'svelte';
import '../../app.css';
import { initPrefs } from '$lib/prefs';
import { installHelpBus } from '$lib/help-bus';
import Features from './Features.svelte';

installHelpBus('Feature Collection');
initPrefs();
mount(Features, { target: document.body });
