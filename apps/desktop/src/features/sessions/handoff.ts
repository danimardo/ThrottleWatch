let requested: string | null = null;

export function requestSessionReport(sessionId: string): void {
  requested = sessionId;
}

/** The session another screen asked Sessions to open, once. */
export function takeRequestedSessionReport(): string | null {
  const sessionId = requested;
  requested = null;
  return sessionId;
}
