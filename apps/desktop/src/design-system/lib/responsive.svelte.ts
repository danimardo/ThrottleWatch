/**
 * Optional convenience helper for the app shell's own responsive
 * decisions (sidebar vs. BottomBar, showing/hiding CausalRail, the
 * NavigationItem density). The components themselves take a density /
 * boolean prop — they do not read the window size directly, because a
 * design-system component should not assume it owns the whole window.
 * This helper is what wires window width to those props.
 *
 * Usage (Svelte 5 runes, in your app shell):
 *
 *   import { createWidthTracker } from '../design-system/lib/responsive.svelte';
 *   const win = createWidthTracker();
 *   // win.tier is 'compact' | 'medium' | 'expanded', reactive
 *
 *   {#if win.tier === 'compact'}
 *     <BottomBar items={...} />
 *   {:else}
 *     <nav>
 *       {#each items as item}
 *         <NavigationItem {...item} density={win.tier === 'expanded' ? 'labeled' : 'icon-only'} />
 *       {/each}
 *     </nav>
 *   {/if}
 *   {#if win.tier === 'expanded'}
 *     <CausalRail nodes={...} />
 *   {/if}
 */

import { BREAKPOINTS } from '../tokens/tokens';

export type WidthTier = 'compact' | 'medium' | 'expanded';

export function tierForWidth(width: number): WidthTier {
  if (width < BREAKPOINTS.compact) return 'compact';
  if (width < BREAKPOINTS.expanded) return 'medium';
  return 'expanded';
}

export function createWidthTracker() {
  let width = $state(typeof window !== 'undefined' ? window.innerWidth : BREAKPOINTS.expanded);

  $effect(() => {
    if (typeof window === 'undefined') return;
    const update = () => {
      width = window.innerWidth;
    };
    update();
    window.addEventListener('resize', update);
    return () => {
      window.removeEventListener('resize', update);
    };
  });

  return {
    get width() {
      return width;
    },
    get tier(): WidthTier {
      return tierForWidth(width);
    }
  };
}
