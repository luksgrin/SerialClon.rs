import { mount } from 'svelte';
import '../../app.css';
import { installHelpBus } from '$lib/help-bus';
import Features from './Features.svelte';

installHelpBus('Feature Collection');
mount(Features, { target: document.body });
