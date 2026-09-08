import { mount } from 'svelte';
import '../../app.css';
import { installHelpBus } from '$lib/help-bus';
import Gateway from './Gateway.svelte';

installHelpBus('Gateway Cloning');
mount(Gateway, { target: document.body });
