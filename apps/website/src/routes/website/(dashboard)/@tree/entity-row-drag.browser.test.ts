import { afterEach, describe, expect, it } from 'vitest';
import { EntityRowDragController } from './entity-row-drag.svelte';
import type { PaneGroup } from '../[slug]/@pane/context.svelte';
import type { EntityRowDragItem, EntityRowDrop, EntityRowDropResult } from './entity-row-drag.svelte';

const ROW_HEIGHT = 32;
const ROWS = ['A', 'B', 'C'];

let controller: EntityRowDragController | undefined;
let action: { destroy?: () => void } | undefined;

afterEach(() => {
  action?.destroy?.();
  controller?.destroy();
  action = undefined;
  controller = undefined;
  document.body.replaceChildren();
});

const installPointerCapture = (element: HTMLElement) => {
  let capturedPointerId: number | undefined;
  element.setPointerCapture = (pointerId) => (capturedPointerId = pointerId);
  element.hasPointerCapture = (pointerId) => capturedPointerId === pointerId;
  element.releasePointerCapture = (pointerId) => {
    if (capturedPointerId === pointerId) capturedPointerId = undefined;
  };
};

const pointer = (
  target: EventTarget,
  type: 'pointerdown' | 'pointermove' | 'pointerup',
  { x = 20, y = 20, pointerId = 1 }: { x?: number; y?: number; pointerId?: number } = {},
) => {
  target.dispatchEvent(
    new PointerEvent(type, {
      bubbles: true,
      cancelable: true,
      isPrimary: true,
      button: type === 'pointermove' ? -1 : 0,
      buttons: type === 'pointerup' ? 0 : 1,
      clientX: x,
      clientY: y,
      pointerId,
      pointerType: 'mouse',
    }),
  );
};

const wait = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

const ghostCue = () => controller?.ghost?.cue ?? null;

type Fixture = {
  section: HTMLElement;
  list: HTMLElement;
  rows: HTMLElement[];
  drops: { drop: EntityRowDropResult; item: EntityRowDragItem }[];
  paneExecutions: string[];
  paneZoneAt: (x: number, y: number) => boolean;
};

const mountFixture = ({ paneZoneAt = () => false, holdMs = 40 }: { paneZoneAt?: Fixture['paneZoneAt']; holdMs?: number } = {}) => {
  const surface = document.createElement('div');
  surface.dataset.entityRowDragScrollSurface = '';
  Object.assign(surface.style, { position: 'fixed', top: '0', left: '0', width: '300px', height: '600px' });

  const section = document.createElement('section');
  Object.assign(section.style, { position: 'absolute', top: '0', left: '0', width: '300px', height: `${ROW_HEIGHT * 4}px` });
  const header = document.createElement('div');
  Object.assign(header.style, { height: `${ROW_HEIGHT}px` });
  const list = document.createElement('ul');
  Object.assign(list.style, { margin: '0', padding: '0', listStyle: 'none' });

  const rows = ROWS.map((id) => {
    const li = document.createElement('li');
    const row = document.createElement('a');
    row.dataset.id = id;
    Object.assign(row.style, { display: 'block', height: `${ROW_HEIGHT}px` });
    row.textContent = `Document ${id}`;
    li.append(row);
    list.append(li);
    return row;
  });

  section.append(header, list);
  surface.append(section);
  document.body.append(surface);

  const drops: Fixture['drops'] = [];
  const paneExecutions: string[] = [];
  let activeZone: PaneGroup['activeZone'] = null;

  const paneGroup = {
    get activeZone() {
      return activeZone;
    },
    set activeZone(value: PaneGroup['activeZone']) {
      activeZone = value;
    },
    updateActiveZone(x: number, y: number) {
      activeZone = paneZoneAt(x, y) ? { paneId: 'pane-1', dropZone: 'center' } : null;
    },
    executeDrop(item: { slug: string }) {
      paneExecutions.push(item.slug);
      activeZone = null;
      return true;
    },
    cancelDrag() {
      activeZone = null;
    },
  } as unknown as PaneGroup;

  const resolveDrop = (x: number, y: number, item: EntityRowDragItem): EntityRowDrop | null => {
    const hit = document.elementFromPoint(x, y);
    if (hit && list.contains(hit)) {
      const row = hit.closest<HTMLElement>('[data-id]');
      if (!row?.dataset.id || row.dataset.id === item.id) return null;
      const rect = row.getBoundingClientRect();
      const before = y < rect.top + rect.height / 2;
      const rest = ROWS.filter((id) => id !== item.id);
      const insertAt = rest.indexOf(row.dataset.id) + (before ? 0 : 1);
      return { kind: 'reorder', lowerOrder: rest[insertAt - 1] ?? null, upperOrder: rest[insertAt] ?? null };
    }
    if (hit && section.contains(hit)) return null;
    return { kind: 'outside' };
  };

  controller = new EntityRowDragController({
    paneGroup,
    resolveDrop,
    onDrop: (drop, item) => {
      drops.push({ drop, item });
    },
    holdOutside: { ms: holdMs, cue: '고정 해제' },
  });

  installPointerCapture(rows[0]);
  action = controller.drag(rows[0], { id: 'A', type: 'document', slug: 'document-a', name: 'Document A' }) as { destroy?: () => void };

  return { section, list, rows, drops, paneExecutions, paneZoneAt } satisfies Fixture;
};

const beginDrag = (row: HTMLElement) => {
  pointer(row, 'pointerdown', { x: 20, y: ROW_HEIGHT + 10 });
  pointer(row, 'pointermove', { x: 31, y: ROW_HEIGHT + 10 });
};

describe('entity row drag controller', () => {
  it('resolves a drop between rows as reorder with the neighbour orders', () => {
    const fixture = mountFixture();
    beginDrag(fixture.rows[0]);
    expect(controller?.active).toBe(true);

    pointer(fixture.rows[0], 'pointermove', { x: 20, y: ROW_HEIGHT * 3 + 4 });
    expect(controller?.drop).toEqual({ kind: 'reorder', lowerOrder: 'B', upperOrder: 'C' });

    pointer(fixture.rows[0], 'pointerup', { x: 20, y: ROW_HEIGHT * 3 + 4 });
    expect(fixture.drops).toEqual([
      { drop: { kind: 'reorder', lowerOrder: 'B', upperOrder: 'C' }, item: expect.objectContaining({ id: 'A' }) },
    ]);
    expect(controller?.drop).toBeNull();
    expect(controller?.ghost).toBeNull();
    expect(controller?.active).toBe(false);
  });

  it('arms unpin after holding outside the section and shows the cue on the ghost', async () => {
    const fixture = mountFixture({ holdMs: 40 });
    beginDrag(fixture.rows[0]);

    pointer(fixture.rows[0], 'pointermove', { x: 20, y: 400 });
    expect(ghostCue()).toBeNull();
    await wait(70);
    expect(ghostCue()).toBe('고정 해제');

    pointer(fixture.rows[0], 'pointerup', { x: 20, y: 400 });
    expect(fixture.drops).toEqual([{ drop: { kind: 'unpin' }, item: expect.objectContaining({ id: 'A' }) }]);
  });

  it('cancels the hold when the pointer returns inside the section before it arms', async () => {
    const fixture = mountFixture({ holdMs: 60 });
    beginDrag(fixture.rows[0]);

    pointer(fixture.rows[0], 'pointermove', { x: 20, y: 400 });
    await wait(20);
    pointer(fixture.rows[0], 'pointermove', { x: 20, y: 8 });
    await wait(80);
    expect(ghostCue()).toBeNull();

    pointer(fixture.rows[0], 'pointerup', { x: 20, y: 8 });
    expect(fixture.drops).toEqual([]);
  });

  it('drops the cue again when leaving the section after it armed and coming back', async () => {
    const fixture = mountFixture({ holdMs: 40 });
    beginDrag(fixture.rows[0]);

    pointer(fixture.rows[0], 'pointermove', { x: 20, y: 400 });
    await wait(70);
    expect(ghostCue()).toBe('고정 해제');

    pointer(fixture.rows[0], 'pointermove', { x: 20, y: 8 });
    expect(ghostCue()).toBeNull();

    pointer(fixture.rows[0], 'pointerup', { x: 20, y: 8 });
    expect(fixture.drops).toEqual([]);
  });

  it('prefers a pane drop zone over the outside hold', async () => {
    const fixture = mountFixture({ holdMs: 40, paneZoneAt: (x) => x > 250 });
    beginDrag(fixture.rows[0]);

    pointer(fixture.rows[0], 'pointermove', { x: 280, y: 400 });
    await wait(70);
    expect(ghostCue()).toBeNull();

    pointer(fixture.rows[0], 'pointerup', { x: 280, y: 400 });
    expect(fixture.paneExecutions).toEqual(['document-a']);
    expect(fixture.drops).toEqual([]);
  });

  it('keeps pane-only behaviour when no resolver is configured', () => {
    const surface = document.createElement('div');
    surface.dataset.entityRowDragScrollSurface = '';
    const row = document.createElement('a');
    row.textContent = 'Document A';
    surface.append(row);
    document.body.append(surface);
    installPointerCapture(row);

    const executions: string[] = [];
    let activeZone: PaneGroup['activeZone'] = null;
    const paneGroup = {
      get activeZone() {
        return activeZone;
      },
      set activeZone(value: PaneGroup['activeZone']) {
        activeZone = value;
      },
      updateActiveZone() {
        activeZone = { paneId: 'pane-1', dropZone: 'center' };
      },
      executeDrop(item: { slug: string }) {
        executions.push(item.slug);
        activeZone = null;
        return true;
      },
      cancelDrag() {
        activeZone = null;
      },
    } as unknown as PaneGroup;

    controller = new EntityRowDragController({ paneGroup });
    action = controller.drag(row, { id: 'A', type: 'document', slug: 'document-a', name: 'Document A' }) as { destroy?: () => void };

    pointer(row, 'pointerdown', { x: 20, y: 20 });
    pointer(row, 'pointermove', { x: 31, y: 20 });
    expect(controller?.active).toBe(true);
    pointer(row, 'pointerup', { x: 31, y: 20 });

    expect(executions).toEqual(['document-a']);
  });
});
