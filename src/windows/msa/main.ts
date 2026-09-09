import { mount } from 'svelte';
import '../../app.css';
import { installHelpBus } from '$lib/help-bus';
import Msa from './Msa.svelte';

installHelpBus('Align Multiple Sequences');
mount(Msa, { target: document.body });
