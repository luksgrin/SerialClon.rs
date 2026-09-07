import { mount } from 'svelte';
import '../../app.css';
import { installHelpBus } from '$lib/help-bus';
import SequenceMap from './SequenceMap.svelte';

installHelpBus('Sequence Map');
mount(SequenceMap, { target: document.body });
