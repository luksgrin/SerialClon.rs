import { mount } from 'svelte';
import '../../app.css';
import { initPrefs } from '$lib/prefs';
import { installHelpBus } from '$lib/help-bus';
import { installEditMenu } from '$lib/edit-menu';
import EnzymeLibrary from './EnzymeLibrary.svelte';

installHelpBus('Enzyme Library');
installEditMenu();
initPrefs();
mount(EnzymeLibrary, { target: document.body });
