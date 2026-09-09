import { mount } from 'svelte';
import '../../app.css';
import { initPrefs } from '$lib/prefs';
import { installHelpBus } from '$lib/help-bus';
import Align from './Align.svelte';

installHelpBus('Align two Sequences');
initPrefs();
mount(Align, { target: document.body });
