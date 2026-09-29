import { mount } from 'svelte';
import '../../app.css';
import { initPrefs } from '$lib/prefs';
import { installHelpBus } from '$lib/help-bus';
import { installEditMenu } from '$lib/edit-menu';
import CodonUsage from './CodonUsage.svelte';

installHelpBus('Codon Usage');
installEditMenu();
initPrefs();
mount(CodonUsage, { target: document.body });
