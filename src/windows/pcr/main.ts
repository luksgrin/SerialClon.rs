import { mount } from 'svelte';
import '../../app.css';
import { initPrefs } from '$lib/prefs';
import { installHelpBus } from '$lib/help-bus';
import Pcr from './Pcr.svelte';

installHelpBus('Run a PCR');
initPrefs();
mount(Pcr, { target: document.body });
