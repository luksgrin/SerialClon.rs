import { mount } from 'svelte';
import '../../app.css';
import { initPrefs } from '$lib/prefs';
import { installHelpBus } from '$lib/help-bus';
import { installEditMenu } from '$lib/edit-menu';
import Features from './Features.svelte';

installHelpBus('Feature Collection');
installEditMenu();
initPrefs();
mount(Features, { target: document.body });
