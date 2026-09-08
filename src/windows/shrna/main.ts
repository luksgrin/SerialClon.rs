import { mount } from 'svelte';
import '../../app.css';
import { installHelpBus } from '$lib/help-bus';
import Shrna from './Shrna.svelte';

installHelpBus('Make shRNA');
mount(Shrna, { target: document.body });
