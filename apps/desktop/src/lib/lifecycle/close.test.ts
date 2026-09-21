import { describe, expect, it } from 'vitest';
import { stopOperationFor } from './close';

describe('stopOperationFor', () => {
  it('stops the guided test', () => {
    expect(stopOperationFor('guided')).toBe(true);
  });

  it('cancels an export or import', () => {
    expect(stopOperationFor('export')).toBe(true);
  });

  it('never needs to stop a download to close', () => {
    expect(stopOperationFor('download')).toBe(false);
  });

  it('has nothing to stop for install either, though the dialog never offers to confirm it', () => {
    expect(stopOperationFor('install')).toBe(false);
  });
});
