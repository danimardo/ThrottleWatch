import { mount } from 'svelte';
import '../../../../design/tokens/tokens.css';
import Bench from './Bench.svelte';

mount(Bench, { target: document.getElementById('app')! });
