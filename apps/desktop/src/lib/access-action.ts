import type { AdvancedAccessState } from '../design-system/lib/access';

export type AccessRequestAction = 'install' | 'upgrade' | 'repair';

/** The `request_low_level_access` action the host must send for an access state (FR-087/089/090). */
export function accessRequestAction(
  state: AdvancedAccessState
): AccessRequestAction | undefined {
  switch (state) {
    case 'installable':
      return 'install';
    case 'upgradable':
      return 'upgrade';
    case 'error':
    case 'denied':
      return 'repair';
    default:
      return undefined;
  }
}
