import { mount } from 'svelte';
import '../../app.css';
import { installHelpBus } from '$lib/help-bus';
import Find from './Find.svelte';

installHelpBus('Find');
mount(Find, { target: document.body });
