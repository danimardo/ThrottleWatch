<script lang="ts">
  /**
   * Reference sheet for the 5 onboarding illustrations — NOT a
   * component this system exports. Confirms they render through the
   * real Svelte components (not just the raw SVG preview) and pick up
   * theme tokens correctly. A real onboarding flow would show one per
   * slide, large, above the slide's title/body copy and the
   * Atrás/Siguiente/Omitir Button row — see AGENTS.md for how slides 1-5
   * map to these five components.
   */
  import {
    OnboardingWelcome,
    OnboardingSignals,
    OnboardingConclusions,
    OnboardingPrivacy,
    OnboardingThisComputer
  } from '../components';

  const slides = [
    { component: OnboardingWelcome, label: '1. Bienvenida' },
    { component: OnboardingSignals, label: '2. Qué observa' },
    { component: OnboardingConclusions, label: '3. Qué puede concluir' },
    { component: OnboardingPrivacy, label: '4. Privacidad y decisiones' },
    { component: OnboardingThisComputer, label: '5. Este equipo' }
  ];
</script>

<div class="tw-ds sheet">
  {#each slides as slide (slide.label)}
    <figure>
      <div class="canvas">
        <slide.component />
      </div>
      <figcaption class="caption" style:color="var(--text-secondary)">{slide.label}</figcaption>
    </figure>
  {/each}
</div>

<style>
  .sheet {
    background: var(--bg);
    padding: var(--space-6);
    display: flex;
    gap: var(--space-5);
    flex-wrap: wrap;
    font-family: var(--font-sans, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif);
  }
  figure {
    margin: 0;
  }
  .canvas {
    width: 280px;
    height: 220px;
    background: var(--surface);
    border: 1px solid var(--hairline);
    border-radius: var(--radius-lg);
  }
  figcaption {
    margin-top: var(--space-2);
    text-align: center;
  }
</style>
