import { mount } from 'svelte';
import '../../app.css';
import { initPrefs } from '$lib/prefs';
import { installHelpBus } from '$lib/help-bus';
import { installEditMenu } from '$lib/edit-menu';
import Gateway from './Gateway.svelte';

installHelpBus('Gateway Cloning');
installEditMenu();
initPrefs();
mount(Gateway, { target: document.body });
