import { mount } from 'svelte';
import '../../app.css';
import { initPrefs } from '$lib/prefs';
import { installHelpBus } from '$lib/help-bus';
import { installEditMenu } from '$lib/edit-menu';
import Shrna from './Shrna.svelte';

installHelpBus('Make shRNA');
installEditMenu();
initPrefs();
mount(Shrna, { target: document.body });
