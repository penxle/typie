import { describe, expect, it, vi } from 'vitest';
import { restoreReviewRound, restoreVisible, ZenModeController } from './zen-mode.svelte';
import type { AppContext, AppPreference, ZenModeRestoreState } from '@typie/ui/context';
import type { PaneGroup, PaneGroupState } from './[slug]/@pane/types';

const { writeReviewRoundSelection } = vi.hoisted(() => ({ writeReviewRoundSelection: vi.fn() }));
vi.mock('$lib/prism/review-round-selection', () => ({ writeReviewRoundSelection }));

const restoreState = (overrides: Partial<ZenModeRestoreState> = {}): ZenModeRestoreState => ({
  version: 1,
  sidebarHidden: false,
  prismPanelOpen: true,
  panelExpandedBySiteAndPaneId: { site: { first: true, deleted: true } },
  reviewRoundByPaneAndDocumentId: {},
  ...overrides,
});

const preference = (overrides: Partial<AppPreference> = {}) =>
  ({
    sidebarHidden: false,
    prismPanelOpen: true,
    zenModeEnabled: false,
    zenModeRestoreState: null,
    ...overrides,
  }) as AppPreference;

const fixture = (initialPreference: AppPreference = preference()) => {
  const app = {
    preference: { current: initialPreference },
    state: { sidebarPeek: true },
  } as unknown as Pick<AppContext, 'preference' | 'state'>;
  const paneState: PaneGroupState = {
    root: { id: 'first', type: 'pane', kind: 'entity', slug: 'document' },
    focusedPaneId: 'first',
    panelExpandedByPaneId: { first: true },
    panelTabByPaneId: {},
    toolbarExpandedByPaneId: {},
  };
  const restored: [string, Record<string, boolean>][] = [];
  const paneGroup = {
    currentSiteId: 'site',
    panes: [{ id: 'first', type: 'pane', kind: 'entity', slug: 'document' }],
    state: { current: paneState },
    readPanelExpandedByPaneId: () => ({ ...paneState.panelExpandedByPaneId }),
    restorePanelExpandedByPaneId: (siteId: string, entry: Record<string, boolean>) => {
      restored.push([siteId, entry]);
      const next = { ...paneState.panelExpandedByPaneId };
      for (const [paneId, wasOpen] of Object.entries(entry)) {
        if (paneId === 'first') next[paneId] = wasOpen || (next[paneId] ?? false);
      }
      paneState.panelExpandedByPaneId = next;
    },
  } as unknown as PaneGroup;
  const track = vi.fn();
  const controller = new ZenModeController({ app, paneGroup, editorRegistry: { get: vi.fn() }, track });
  return { app, controller, paneGroup, paneState, restored, track };
};

describe('focus-mode restore rules', () => {
  it.each([
    [false, false, false],
    [false, true, true],
    [true, false, true],
    [true, true, true],
  ])('restores entry %s OR current %s as %s', (entry, current, expected) => {
    expect(restoreVisible(entry, current)).toBe(expected);
  });

  it('keeps an in-mode review selection and otherwise restores the entry round', () => {
    expect(restoreReviewRound('entry', 'current')).toBe('current');
    expect(restoreReviewRound('entry', null)).toBe('entry');
    expect(restoreReviewRound(null, null)).toBeNull();
  });
});

describe('ZenModeController', () => {
  it('captures and closes participants once, then restores entry OR current state', async () => {
    const { app, controller, paneState, restored, track } = fixture();

    await controller.enter('shortcut');
    expect(app.preference.current).toMatchObject({ sidebarHidden: true, prismPanelOpen: false, zenModeEnabled: true });
    expect(paneState.panelExpandedByPaneId.first).toBe(false);
    expect(app.preference.current.zenModeRestoreState).toMatchObject({ sidebarHidden: false, prismPanelOpen: true });

    app.preference.current.prismPanelOpen = false;
    app.preference.current.sidebarHidden = false;
    await controller.exit('esc');

    expect(app.preference.current).toMatchObject({
      sidebarHidden: false,
      prismPanelOpen: true,
      zenModeEnabled: false,
      zenModeRestoreState: null,
    });
    expect(restored).toEqual([['site', { first: true }]]);
    expect(track.mock.calls).toEqual([
      ['zen_mode_enabled', { via: 'shortcut' }],
      ['zen_mode_disabled', { via: 'esc' }],
    ]);

    await controller.exit('esc');
    expect(track).toHaveBeenCalledTimes(2);
  });

  it('preserves current visibility on reload with a valid snapshot', async () => {
    const snapshot = restoreState({ panelExpandedBySiteAndPaneId: { site: { first: true } } });
    const { app, controller, paneState, track } = fixture(
      preference({ sidebarHidden: false, prismPanelOpen: true, zenModeEnabled: true, zenModeRestoreState: snapshot }),
    );
    paneState.panelExpandedByPaneId.first = true;

    await controller.restoreAfterReload();

    expect(app.preference.current.zenModeRestoreState).toBe(snapshot);
    expect(paneState.panelExpandedByPaneId.first).toBe(true);
    expect(track).not.toHaveBeenCalled();
  });

  it('captures and closes once when an active preference has no restore snapshot', async () => {
    const { app, controller, paneState, track } = fixture(preference({ zenModeEnabled: true, zenModeRestoreState: null }));

    await controller.restoreAfterReload();
    await controller.restoreAfterReload();

    expect(app.preference.current.zenModeRestoreState).toMatchObject({ version: 1, sidebarHidden: false, prismPanelOpen: true });
    expect(app.preference.current).toMatchObject({ sidebarHidden: true, prismPanelOpen: false, zenModeEnabled: true });
    expect(paneState.panelExpandedByPaneId.first).toBe(false);
    expect(track).not.toHaveBeenCalled();
  });

  it('captures and closes a pane first seen during the active session', () => {
    const snapshot = restoreState({ panelExpandedBySiteAndPaneId: { site: { first: false } } });
    const { app, controller, paneGroup, paneState } = fixture(
      preference({ sidebarHidden: true, prismPanelOpen: false, zenModeEnabled: true, zenModeRestoreState: snapshot }),
    );
    paneGroup.panes.push({ id: 'second', type: 'pane', kind: 'home' });
    paneState.panelExpandedByPaneId.second = true;

    controller.syncPanePanels();

    expect(paneState.panelExpandedByPaneId.second).toBe(false);
    expect(app.preference.current.zenModeRestoreState?.panelExpandedBySiteAndPaneId.site?.second).toBe(true);
  });

  it('restores the review round independently for each pane showing the same document', async () => {
    const { controller } = fixture();
    let first: string | null = 'first-entry';
    let second: string | null = 'second-entry';
    const firstParticipant = {
      paneId: 'first',
      documentId: 'document',
      ready: () => true,
      selectedRoundId: () => first,
      roundIds: () => ['first-entry', 'second-entry', 'second-current'],
      applySelection: (roundId: string | null) => (first = roundId),
    };
    const secondParticipant = {
      ...firstParticipant,
      paneId: 'second',
      selectedRoundId: () => second,
      applySelection: (roundId: string | null) => (second = roundId),
    };
    controller.registerReview(firstParticipant);
    controller.registerReview(secondParticipant);

    await controller.enter('pane_header');
    expect([first, second]).toEqual([null, null]);

    second = 'second-current';
    await controller.exit('pane_header');
    expect([first, second]).toEqual(['first-entry', 'second-current']);
  });

  it('restores the persisted review round after its pane was unmounted in focus mode', async () => {
    const { controller } = fixture();
    let selected: string | null = 'entry';
    const unregister = controller.registerReview({
      paneId: 'first',
      documentId: 'document',
      ready: () => true,
      selectedRoundId: () => selected,
      roundIds: () => ['entry'],
      applySelection: (roundId) => (selected = roundId),
    });

    await controller.enter('pane_header');
    expect(selected).toBeNull();
    unregister();
    writeReviewRoundSelection.mockClear();

    await controller.exit('pane_header');

    expect(writeReviewRoundSelection).toHaveBeenCalledWith('document', 'entry');
  });

  it('persists the current review round from a live pane even when another pane is focused', async () => {
    const { controller } = fixture();
    let selected: string | null = 'entry';
    controller.registerReview({
      paneId: 'second',
      documentId: 'document',
      ready: () => true,
      selectedRoundId: () => selected,
      roundIds: () => ['entry', 'current'],
      applySelection: (roundId) => (selected = roundId),
    });

    await controller.enter('pane_header');
    selected = 'current';
    writeReviewRoundSelection.mockClear();

    await controller.exit('pane_header');

    expect(writeReviewRoundSelection).toHaveBeenCalledWith('document', 'current');
  });

  it('drops an entry review round that no longer exists', async () => {
    const { controller } = fixture();
    let selected: string | null = 'deleted';
    controller.registerReview({
      paneId: 'first',
      documentId: 'document',
      ready: () => true,
      selectedRoundId: () => selected,
      roundIds: () => ['remaining'],
      applySelection: (roundId) => (selected = roundId),
    });

    await controller.enter('command_palette');
    await controller.exit('command_palette');
    expect(selected).toBeNull();
  });
});
