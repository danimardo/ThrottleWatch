import { CURRENT_NOTICE_VERSION } from './model';

export interface NoticeDefinition {
  id: string;
  version: number;
  titleKey: string;
  bodyKey: string;
}

/** Versioned notices stay independent from the resolved onboarding state. */
export const NOTICE_DEFINITIONS: readonly NoticeDefinition[] = [
  {
    id: 'low-level-access',
    version: CURRENT_NOTICE_VERSION,
    titleKey: 'notices.lowLevel.title',
    bodyKey: 'notices.lowLevel.body'
  }
];

export function pendingNoticeDefinitions(
  lastSeenVersion: number
): readonly NoticeDefinition[] {
  return NOTICE_DEFINITIONS.filter(
    (notice) => notice.version > lastSeenVersion
  );
}
