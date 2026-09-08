import { mount } from 'svelte';
import '../../app.css';
import { installHelpBus } from '$lib/help-bus';
import Align from './Align.svelte';

installHelpBus('Align two Sequences');
mount(Align, { target: document.body });
