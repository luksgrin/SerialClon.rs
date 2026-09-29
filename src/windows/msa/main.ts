import { mount } from 'svelte';
import '../../app.css';
import { initPrefs } from '$lib/prefs';
import { installHelpBus } from '$lib/help-bus';
import { installEditMenu } from '$lib/edit-menu';
import Msa from './Msa.svelte';

installHelpBus('Align Multiple Sequences');
installEditMenu();
initPrefs();
mount(Msa, { target: document.body });
