import { mount } from 'svelte';
import '../../app.css';
import { installHelpBus } from '$lib/help-bus';
import EnzymeLibrary from './EnzymeLibrary.svelte';

installHelpBus('Enzyme Library');
mount(EnzymeLibrary, { target: document.body });
