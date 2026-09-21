import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';
import DialogFixture from './test-fixtures/DialogFixture.svelte';
import SegmentedFixture from './test-fixtures/SegmentedFixture.svelte';
import AnalysisChartFixture from './test-fixtures/AnalysisChartFixture.svelte';
import CpuTableFixture from './test-fixtures/CpuTableFixture.svelte';
import OnboardingFixture from './test-fixtures/OnboardingFixture.svelte';

describe('design system behavior', () => {
  afterEach(() => cleanup());

  it('opens Dialog, moves focus into it and closes on native close/Escape path', async () => {
    const { container } = render(DialogFixture);
    const trigger = screen.getByRole('button', { name: 'Abrir diálogo' });
    trigger.focus();
    await fireEvent.click(trigger);
    const dialog = container.querySelector('dialog');
    expect(dialog).toHaveAttribute('open');
    expect(document.activeElement).toBe(screen.getByRole('button', { name: 'Cancelar' }));
    await fireEvent.keyDown(dialog!, { key: 'Escape' });
    dialog!.close();
    expect(dialog).not.toHaveAttribute('open');
  });

  it('changes SegmentedControl selection from the keyboard focus target', async () => {
    render(SegmentedFixture);
    const options = screen.getAllByRole('radio');
    options[1].focus();
    await fireEvent.keyDown(options[1], { key: 'Enter' });
    await fireEvent.click(options[1]);
    expect(options[1]).toHaveAttribute('aria-checked', 'true');
    expect(screen.getByTestId('selected')).toHaveTextContent('two');
  });

  it('moves the AnalysisChart cursor and commits a keyboard range', async () => {
    render(AnalysisChartFixture);
    const slider = screen.getByRole('slider', { name: 'Cursor' });
    expect(slider).toHaveAttribute('aria-valuenow', '0');
    await fireEvent.keyDown(slider, { key: 'ArrowRight' });
    expect(slider).toHaveAttribute('aria-valuenow', '1');
    await fireEvent.keyDown(slider, { key: 'Enter' });
    await fireEvent.keyDown(slider, { key: 'End' });
    await fireEvent.keyDown(slider, { key: 'Enter' });
    expect(screen.getByTestId('selected-range')).toHaveTextContent('1-3');
  });

  it('sorts CpuAdvancedTable rows by numeric temperature', async () => {
    const { container } = render(CpuTableFixture);
    const temperature = container.querySelector<HTMLButtonElement>('.header-row .col-num');
    expect(temperature).not.toBeNull();
    await fireEvent.click(temperature!);
    const firstRow = container.querySelector('.data-row .col-index');
    expect(firstRow).toHaveTextContent('1');
  });

  it('advances the OnboardingFlow state machine and finishes on the fifth step', async () => {
    render(OnboardingFixture);
    const next = () => screen.getByRole('button', { name: 'Siguiente' });
    for (let i = 0; i < 4; i += 1) await fireEvent.click(next());
    expect(screen.getByText('Paso cinco')).toBeInTheDocument();
    await fireEvent.click(screen.getByRole('button', { name: 'Terminar' }));
    expect(screen.getByTestId('finished')).toHaveTextContent('sí');
  });
});
