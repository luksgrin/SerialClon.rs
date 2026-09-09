import { mount } from 'svelte';
import '../../app.css';
import { initPrefs } from '$lib/prefs';
import { installHelpBus } from '$lib/help-bus';
import SequenceMap from './SequenceMap.svelte';

installHelpBus('Sequence Map');
initPrefs();
mount(SequenceMap, { target: document.body });
