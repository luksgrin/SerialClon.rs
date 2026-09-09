import { mount } from 'svelte';
import '../../app.css';
import { initPrefs } from '$lib/prefs';
import { installHelpBus } from '$lib/help-bus';
import Sequence from './Sequence.svelte';

installHelpBus('Sequence Window');
initPrefs();
mount(Sequence, { target: document.body });
