import { mount } from 'svelte';
import '../../app.css';
import { installHelpBus } from '$lib/help-bus';
import Sequence from './Sequence.svelte';

installHelpBus('Sequence Window');
mount(Sequence, { target: document.body });
