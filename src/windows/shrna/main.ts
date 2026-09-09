import { mount } from 'svelte';
import '../../app.css';
import { initPrefs } from '$lib/prefs';
import { installHelpBus } from '$lib/help-bus';
import Shrna from './Shrna.svelte';

installHelpBus('Make shRNA');
initPrefs();
mount(Shrna, { target: document.body });
