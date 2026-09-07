import { mount } from 'svelte';
import '../../app.css';
import { installHelpBus } from '$lib/help-bus';
import SilentMap from './SilentMap.svelte';

installHelpBus('Silent Map');
mount(SilentMap, { target: document.body });
