import { documentEditing } from './state.svelte';

// Application navigation can await preparation. Browser unload cannot: warn
// synchronously, and use page visibility changes for best-effort local capture.
export function guardBrowserUnload(): () => void {
  const events = new AbortController();
  let unloadGuardEnabled = false;
  let pendingCapture: Promise<void> | undefined;
  let showDetailsOnInteraction = false;

  const captureInBackground = () => {
    pendingCapture ??= documentEditing.capture().finally(() => {
      pendingCapture = undefined;
    });
  };
  const beforeUnload = (event: BeforeUnloadEvent) => {
    if (!documentEditing.hasUnprotectedChanges()) return;
    event.preventDefault();
    event.returnValue = true;
    showDetailsOnInteraction = true;
    captureInBackground();
  };
  const updateUnloadGuard = () => {
    const needed = documentEditing.hasUnprotectedChanges();
    if (needed && !unloadGuardEnabled) window.addEventListener('beforeunload', beforeUnload, { signal: events.signal });
    else if (!needed && unloadGuardEnabled) window.removeEventListener('beforeunload', beforeUnload);
    unloadGuardEnabled = needed;
    if (!needed) showDetailsOnInteraction = false;
  };
  const offerSaveDetails = () => {
    // Actual continued interaction, not a timer/focus event, is sufficient to
    // offer help on this still-live page. Never replay the abandoned departure.
    if (!showDetailsOnInteraction) return;
    showDetailsOnInteraction = false;
    const sessions = documentEditing.sessions.filter((session) => !session.isProtected());
    if (sessions.length > 0) void documentEditing.showSaveStatus(sessions);
  };
  const unsubscribe = documentEditing.onChange(updateUnloadGuard);
  updateUnloadGuard();
  document.addEventListener('visibilitychange', captureInBackground, { signal: events.signal });
  window.addEventListener('pagehide', captureInBackground, { signal: events.signal });
  window.addEventListener('pageshow', captureInBackground, { signal: events.signal });
  window.addEventListener('pointerdown', offerSaveDetails, { capture: true, signal: events.signal });
  window.addEventListener('keydown', offerSaveDetails, { capture: true, signal: events.signal });
  return () => {
    unsubscribe();
    events.abort();
  };
}
