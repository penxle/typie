import { createStableContext } from '@typie/ui/context/stable';
import mixpanel from 'mixpanel-browser';
import { tick } from 'svelte';
import { writeReviewRoundSelection } from '$lib/prism/review-round-selection';
import type { AppContext, ZenModeRestoreState } from '@typie/ui/context';
import type { PaneGroup } from './[slug]/@pane/context.svelte';
import type { RegisteredEditor } from './[slug]/@pane/editor-registry.svelte';

export type ZenModeVia = 'shortcut' | 'command_palette' | 'pane_header' | 'esc';

export type ZenModeReviewParticipant = {
  paneId: string;
  documentId: string;
  ready: () => boolean;
  selectedRoundId: () => string | null;
  roundIds: () => readonly string[];
  applySelection: (roundId: string | null) => void;
};

type ReviewRegistration = {
  participant: ZenModeReviewParticipant;
  closeWhenReady: boolean;
};

type EditorRegistry = {
  get(paneId: string, slug: string): RegisteredEditor | undefined;
};

type ZenModeControllerOptions = {
  app: Pick<AppContext, 'preference' | 'state'>;
  paneGroup: PaneGroup;
  editorRegistry: EditorRegistry;
  focusWorkbench?: () => void;
  track?: (event: 'zen_mode_enabled' | 'zen_mode_disabled', properties: { via: ZenModeVia }) => void;
};

const [getZenMode, setZenMode] = createStableContext<ZenModeController>('dashboard.ZenMode');

export { getZenMode };

export const restoreVisible = (entryVisible: boolean, currentVisible: boolean): boolean => entryVisible || currentVisible;

export const restoreReviewRound = (entryRoundId: string | null, currentRoundId: string | null): string | null =>
  currentRoundId ?? entryRoundId;

const emptyRestoreState = (sidebarHidden: boolean, prismPanelOpen: boolean): ZenModeRestoreState => ({
  version: 1,
  sidebarHidden,
  prismPanelOpen,
  panelExpandedBySiteAndPaneId: {},
  reviewRoundByPaneAndDocumentId: {},
});

const isRecord = (value: unknown): value is Record<string, unknown> => typeof value === 'object' && value !== null && !Array.isArray(value);

const isBooleanRecord = (value: unknown): value is Record<string, boolean> =>
  isRecord(value) && Object.values(value).every((entry) => typeof entry === 'boolean');

const isPanelRestoreState = (value: unknown): value is ZenModeRestoreState['panelExpandedBySiteAndPaneId'] =>
  isRecord(value) && Object.values(value).every(isBooleanRecord);

const isReviewRoundRecord = (value: unknown): value is Record<string, string | null> =>
  isRecord(value) && Object.values(value).every((entry) => entry === null || typeof entry === 'string');

const isReviewRestoreState = (value: unknown): value is ZenModeRestoreState['reviewRoundByPaneAndDocumentId'] =>
  isRecord(value) && Object.values(value).every(isReviewRoundRecord);

const isRestoreState = (value: unknown): value is ZenModeRestoreState =>
  isRecord(value) &&
  value.version === 1 &&
  typeof value.sidebarHidden === 'boolean' &&
  typeof value.prismPanelOpen === 'boolean' &&
  isPanelRestoreState(value.panelExpandedBySiteAndPaneId) &&
  isReviewRestoreState(value.reviewRoundByPaneAndDocumentId);

const hasOwn = <T extends object>(value: T, key: PropertyKey): boolean => Object.hasOwn(value, key);

export class ZenModeController {
  #app: ZenModeControllerOptions['app'];
  #paneGroup: PaneGroup;
  #editorRegistry: EditorRegistry;
  #focusWorkbench: () => void;
  #track: NonNullable<ZenModeControllerOptions['track']>;
  // eslint-disable-next-line svelte/prefer-svelte-reactivity -- imperative registration ownership; no render reads this collection directly
  #reviews = new Set<ReviewRegistration>();
  #restoreChecked = false;

  constructor({ app, paneGroup, editorRegistry, focusWorkbench, track }: ZenModeControllerOptions) {
    this.#app = app;
    this.#paneGroup = paneGroup;
    this.#editorRegistry = editorRegistry;
    this.#focusWorkbench = focusWorkbench ?? (() => null);
    this.#track = track ?? ((event, properties) => mixpanel.track(event, properties));
  }

  get active(): boolean {
    return this.#app.preference.current.zenModeEnabled;
  }

  async restoreAfterReload(): Promise<void> {
    if (this.#restoreChecked) return;
    this.#restoreChecked = true;

    const preference = this.#app.preference.current;
    if (!preference.zenModeEnabled) {
      if (preference.zenModeRestoreState !== null) this.#writePreference(false, null);
      return;
    }

    if (isRestoreState(preference.zenModeRestoreState)) return;

    const handoff = this.#needsFocusHandoff();
    const restoreState = this.#captureRestoreState();
    this.#markRegisteredReviewsForClose();
    this.#writePreference(true, restoreState);
    this.#closeGlobalSurfaces();
    this.#closeCurrentPanePanels();
    this.#closeReadyReviews();
    if (handoff) await this.#handoffFocus();
  }

  async enter(via: Exclude<ZenModeVia, 'esc'>): Promise<void> {
    if (this.active) return;

    const handoff = this.#needsFocusHandoff();
    this.#markRegisteredReviewsForClose();
    const restoreState = this.#captureRestoreState();
    this.#writePreference(true, restoreState);
    this.#closeGlobalSurfaces();
    this.#closeCurrentPanePanels();
    this.#closeReadyReviews();
    this.#track('zen_mode_enabled', { via });

    if (handoff) await this.#handoffFocus();
  }

  async exit(via: ZenModeVia): Promise<void> {
    if (!this.active) return;

    const preference = this.#app.preference.current;
    const restoreState = this.#restoreState() ?? this.#captureRestoreState();

    const sidebarHidden = restoreState.sidebarHidden && preference.sidebarHidden;
    const prismPanelOpen = restoreVisible(restoreState.prismPanelOpen, preference.prismPanelOpen);

    for (const [siteId, entry] of Object.entries(restoreState.panelExpandedBySiteAndPaneId)) {
      this.#paneGroup.restorePanelExpandedByPaneId(siteId, entry);
    }

    this.#restoreReviews(restoreState.reviewRoundByPaneAndDocumentId);
    this.#app.state.sidebarPeek = false;
    this.#app.preference.current = {
      ...preference,
      sidebarHidden,
      prismPanelOpen,
      zenModeEnabled: false,
      zenModeRestoreState: null,
    };
    for (const registration of this.#reviews) registration.closeWhenReady = false;
    this.#track('zen_mode_disabled', { via });
  }

  async toggle(via: Exclude<ZenModeVia, 'esc'>): Promise<void> {
    if (this.active) await this.exit(via);
    else await this.enter(via);
  }

  syncPanePanels(): void {
    if (!this.active) return;
    const restoreState = this.#restoreState();
    if (!restoreState) return;

    const siteId = this.#paneGroup.currentSiteId;
    const entry = restoreState.panelExpandedBySiteAndPaneId[siteId] ?? {};
    const current = this.#paneGroup.state.current.panelExpandedByPaneId;
    const missing = this.#paneGroup.panes.filter((pane) => !hasOwn(entry, pane.id));
    if (missing.length === 0) return;

    const nextEntry = { ...entry };
    const nextCurrent = { ...current };
    const handoff = this.#needsFocusHandoff();
    for (const pane of missing) {
      nextEntry[pane.id] = current[pane.id] ?? false;
      nextCurrent[pane.id] = false;
    }

    this.#persistRestoreState({
      ...restoreState,
      panelExpandedBySiteAndPaneId: { ...restoreState.panelExpandedBySiteAndPaneId, [siteId]: nextEntry },
    });
    this.#paneGroup.state.current.panelExpandedByPaneId = nextCurrent;
    if (handoff) void this.#handoffFocus();
  }

  registerReview(participant: ZenModeReviewParticipant): () => void {
    const restoreState = this.#restoreState();
    const registration: ReviewRegistration = {
      participant,
      closeWhenReady:
        this.active &&
        !(restoreState && hasOwn(restoreState.reviewRoundByPaneAndDocumentId[participant.paneId] ?? {}, participant.documentId)),
    };
    this.#reviews.add(registration);
    this.syncReview(participant);
    return () => this.#reviews.delete(registration);
  }

  syncReview(participant: ZenModeReviewParticipant): void {
    if (!this.active || !participant.ready()) return;
    const registration = [...this.#reviews].find((candidate) => candidate.participant === participant);
    if (!registration) return;

    let restoreState = this.#restoreState();
    if (!restoreState) return;

    const documentId = participant.documentId;
    const paneEntry = restoreState.reviewRoundByPaneAndDocumentId[participant.paneId] ?? {};
    if (!hasOwn(paneEntry, documentId)) {
      restoreState = {
        ...restoreState,
        reviewRoundByPaneAndDocumentId: {
          ...restoreState.reviewRoundByPaneAndDocumentId,
          [participant.paneId]: { ...paneEntry, [documentId]: participant.selectedRoundId() },
        },
      };
      this.#persistRestoreState(restoreState);
      registration.closeWhenReady = true;
    }

    if (!registration.closeWhenReady) return;
    const handoff = this.#needsFocusHandoff();
    registration.closeWhenReady = false;
    participant.applySelection(null);
    writeReviewRoundSelection(documentId, null);
    if (handoff) void this.#handoffFocus();
  }

  // eslint-disable-next-line unicorn/consistent-class-member-order -- private lifecycle helpers stay together after the public command surface
  #restoreState(): ZenModeRestoreState | null {
    const restoreState = this.#app.preference.current.zenModeRestoreState;
    return isRestoreState(restoreState) ? restoreState : null;
  }

  #writePreference(active: boolean, restoreState: ZenModeRestoreState | null): void {
    this.#app.preference.current = {
      ...this.#app.preference.current,
      zenModeEnabled: active,
      zenModeRestoreState: restoreState,
    };
  }

  #persistRestoreState(restoreState: ZenModeRestoreState): void {
    this.#writePreference(true, restoreState);
  }

  #captureRestoreState(): ZenModeRestoreState {
    const preference = this.#app.preference.current;
    const restoreState = emptyRestoreState(preference.sidebarHidden, preference.prismPanelOpen);
    const siteId = this.#paneGroup.currentSiteId;
    const panels = this.#paneGroup.readPanelExpandedByPaneId(siteId);
    restoreState.panelExpandedBySiteAndPaneId[siteId] = Object.fromEntries(
      this.#paneGroup.panes.map((pane) => [pane.id, panels[pane.id] ?? false]),
    );

    for (const { participant } of this.#reviews) {
      if (!participant.ready()) continue;
      const paneEntry = restoreState.reviewRoundByPaneAndDocumentId[participant.paneId] ?? {};
      if (hasOwn(paneEntry, participant.documentId)) continue;
      restoreState.reviewRoundByPaneAndDocumentId[participant.paneId] = {
        ...paneEntry,
        [participant.documentId]: participant.selectedRoundId(),
      };
    }
    return restoreState;
  }

  #closeGlobalSurfaces(): void {
    this.#app.state.sidebarPeek = false;
    this.#app.preference.current = {
      ...this.#app.preference.current,
      sidebarHidden: true,
      prismPanelOpen: false,
    };
  }

  #closeCurrentPanePanels(): void {
    const current = this.#paneGroup.state.current.panelExpandedByPaneId;
    const closed = Object.fromEntries(Object.keys(current).map((paneId) => [paneId, false]));
    for (const pane of this.#paneGroup.panes) closed[pane.id] = false;
    this.#paneGroup.state.current.panelExpandedByPaneId = closed;
  }

  #markRegisteredReviewsForClose(): void {
    for (const registration of this.#reviews) registration.closeWhenReady = true;
  }

  #closeReadyReviews(): void {
    // eslint-disable-next-line svelte/prefer-svelte-reactivity -- local deduplication for one close transaction
    const documentIds = new Set<string>();
    for (const registration of this.#reviews) {
      if (!registration.closeWhenReady || !registration.participant.ready()) continue;
      registration.closeWhenReady = false;
      registration.participant.applySelection(null);
      documentIds.add(registration.participant.documentId);
    }
    for (const documentId of documentIds) writeReviewRoundSelection(documentId, null);
  }

  #restoreReviews(entryByPaneAndDocumentId: ZenModeRestoreState['reviewRoundByPaneAndDocumentId']): void {
    // eslint-disable-next-line svelte/prefer-svelte-reactivity -- local deduplication for one restore transaction
    const persistedByDocumentId = new Map<string, string | null>();
    // eslint-disable-next-line svelte/prefer-svelte-reactivity -- local precedence tracking for one restore transaction
    const restoredLiveDocumentIds = new Set<string>();
    for (const paneEntry of Object.values(entryByPaneAndDocumentId)) {
      for (const [documentId, roundId] of Object.entries(paneEntry)) {
        if (!persistedByDocumentId.has(documentId)) persistedByDocumentId.set(documentId, roundId);
      }
    }

    for (const { participant } of this.#reviews) {
      if (!participant.ready()) continue;
      const paneEntry = entryByPaneAndDocumentId[participant.paneId];
      if (!paneEntry || !hasOwn(paneEntry, participant.documentId)) continue;

      const entryRoundId = paneEntry[participant.documentId] ?? null;
      const currentRoundId = participant.selectedRoundId();
      let resolved = restoreReviewRound(entryRoundId, currentRoundId);
      if (resolved !== null && !participant.roundIds().includes(resolved)) resolved = null;
      participant.applySelection(resolved);

      if (!restoredLiveDocumentIds.has(participant.documentId) || participant.paneId === this.#paneGroup.state.current.focusedPaneId) {
        persistedByDocumentId.set(participant.documentId, resolved);
        restoredLiveDocumentIds.add(participant.documentId);
      }
    }
    for (const [documentId, roundId] of persistedByDocumentId) writeReviewRoundSelection(documentId, roundId);
  }

  #needsFocusHandoff(): boolean {
    if (typeof document === 'undefined') return false;
    const active = document.activeElement;
    if (!(active instanceof HTMLElement)) return true;
    if (active.closest('[data-zen-mode-closing-surface], [data-zen-mode-pane-chrome]')) return true;
    return active.closest('[data-pane-id]') === null;
  }

  async #handoffFocus(): Promise<void> {
    if (typeof document === 'undefined') return;
    await tick();

    const focusedPaneId = this.#paneGroup.state.current.focusedPaneId;
    const pane = focusedPaneId ? this.#paneGroup.panes.find((candidate) => candidate.id === focusedPaneId) : undefined;
    if (pane?.kind === 'entity') {
      const editor = this.#editorRegistry.get(pane.id, pane.slug);
      if (editor) {
        editor.focus();
        return;
      }
    }

    const escapedPaneId = focusedPaneId ? CSS.escape(focusedPaneId) : null;
    const paneElement = escapedPaneId ? document.querySelector<HTMLElement>(`[data-pane-id="${escapedPaneId}"]`) : null;
    if (paneElement?.isConnected) {
      paneElement.focus({ preventScroll: true });
      return;
    }

    this.#focusWorkbench();
  }
}

export const setupZenMode = (options: ZenModeControllerOptions): ZenModeController => {
  const controller = new ZenModeController(options);
  setZenMode(controller);
  return controller;
};
