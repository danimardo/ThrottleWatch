import { svelte } from '@sveltejs/vite-plugin-svelte';
import { defineConfig } from 'vite';

// This harness's only job is to type-check, build, and browse the
// package one level up (../) exactly as svelte-check/vite build were
// run to verify each batch. Nothing here is app configuration to copy
// into a real project.
export default defineConfig({
  plugins: [svelte()]
});
