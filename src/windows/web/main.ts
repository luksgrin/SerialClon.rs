import { mount } from 'svelte';
import '../../app.css';
import { initPrefs } from '$lib/prefs';
import { installHelpBus } from '$lib/help-bus';
import { installEditMenu } from '$lib/edit-menu';
import Web from './Web.svelte';

installHelpBus('Web Access');
installEditMenu();
initPrefs();
mount(Web, { target: document.body });
