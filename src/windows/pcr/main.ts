import { mount } from 'svelte';
import '../../app.css';
import { installHelpBus } from '$lib/help-bus';
import Pcr from './Pcr.svelte';

installHelpBus('Run a PCR');
mount(Pcr, { target: document.body });
