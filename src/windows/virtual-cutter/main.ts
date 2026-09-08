import { mount } from 'svelte';
import '../../app.css';
import { installHelpBus } from '$lib/help-bus';
import VirtualCutter from './VirtualCutter.svelte';

installHelpBus('Virtual Cutter');
mount(VirtualCutter, { target: document.body });
