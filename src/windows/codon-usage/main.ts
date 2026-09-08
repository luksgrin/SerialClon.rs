import { mount } from 'svelte';
import '../../app.css';
import { installHelpBus } from '$lib/help-bus';
import CodonUsage from './CodonUsage.svelte';

installHelpBus('Codon Usage');
mount(CodonUsage, { target: document.body });
