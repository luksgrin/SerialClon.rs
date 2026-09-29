import { mount } from 'svelte';
import '../../app.css';
import { initPrefs } from '$lib/prefs';
import { installHelpBus } from '$lib/help-bus';
import { installEditMenu } from '$lib/edit-menu';
import SequenceMap from './SequenceMap.svelte';

installHelpBus('Sequence Map');
installEditMenu();
initPrefs();
mount(SequenceMap, { target: document.body });
