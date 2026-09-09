import { mount } from 'svelte';
import '../../app.css';
import { initPrefs } from '$lib/prefs';
import { installHelpBus } from '$lib/help-bus';
import Web from './Web.svelte';

installHelpBus('Web Access');
initPrefs();
mount(Web, { target: document.body });
