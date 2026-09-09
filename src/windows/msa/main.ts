import { mount } from 'svelte';
import '../../app.css';
import { initPrefs } from '$lib/prefs';
import { installHelpBus } from '$lib/help-bus';
import Msa from './Msa.svelte';

installHelpBus('Align Multiple Sequences');
initPrefs();
mount(Msa, { target: document.body });
