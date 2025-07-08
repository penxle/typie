<script lang="ts">
  import { random } from '@ctrl/tinycolor';
  import stringHash from '@sindresorhus/string-hash';
  import { nanoid } from 'nanoid';
  import { base64 } from 'rfc4648';
  import { onMount } from 'svelte';
  import { match } from 'ts-pattern';
  import { IndexeddbPersistence } from 'y-indexeddb';
  import * as YAwareness from 'y-protocols/awareness';
  import * as Y from 'yjs';
  import { CanvasSyncType } from '@/enums';
  import { browser } from '$app/environment';
  import { page } from '$app/state';
  import { graphql } from '$graphql';
  import { getThemeContext } from '$lib/context';
  import { css } from '$styled-system/css';
  import { Canvas } from './lib/canvas.svelte';
  import Panel from './Panel.svelte';
  import Toolbar from './Toolbar.svelte';
  import Zoom from './Zoom.svelte';

  const syncCanvas = graphql(`
    mutation Canvas_SyncCanvas_Mutation($input: SyncCanvasInput!) {
      syncCanvas(input: $input)
    }
  `);

  const canvasSyncStream = graphql(`
    subscription Canvas_CanvasSyncStream_Subscription($clientId: String!, $canvasId: ID!) {
      canvasSyncStream(clientId: $clientId, canvasId: $canvasId) {
        canvasId
        type
        data
      }
    }
  `);

  const clientId = nanoid();
  const canvasId = $derived(page.url.searchParams.get('id') ?? nanoid());

  let container = $state<HTMLDivElement>();
  let canvas = $state<Canvas>();

  const doc = new Y.Doc();
  const awareness = new YAwareness.Awareness(doc);

  const theme = getThemeContext();
  theme.force('light');

  const cursor = $derived.by(() => {
    if (!canvas) return 'default';

    return match(canvas.state.tool)
      .with('pan', () => 'grab')
      .with('select', () => 'default')
      .with('brush', () => 'default')
      .with('arrow', 'line', 'rectangle', 'ellipse', 'stickynote', () => 'crosshair')
      .exhaustive();
  });

  doc.on('updateV2', async (update, origin) => {
    if (browser && origin !== 'remote') {
      await syncCanvas(
        {
          clientId,
          canvasId,
          type: CanvasSyncType.UPDATE,
          data: base64.stringify(update),
        },
        { transport: 'ws' },
      );
    }
  });

  awareness.on('update', async (states: { added: number[]; updated: number[]; removed: number[] }, origin: unknown) => {
    if (browser && origin !== 'remote') {
      const update = YAwareness.encodeAwarenessUpdate(awareness, [...states.added, ...states.updated, ...states.removed]);

      await syncCanvas(
        {
          clientId,
          canvasId,
          type: CanvasSyncType.AWARENESS,
          data: base64.stringify(update),
        },
        { transport: 'ws' },
      );
    }
  });

  const forceSync = async () => {
    const vector = Y.encodeStateVector(doc);

    await syncCanvas(
      {
        clientId,
        canvasId,
        type: CanvasSyncType.VECTOR,
        data: base64.stringify(vector),
      },
      { transport: 'ws' },
    );
  };

  onMount(() => {
    if (!container) {
      return;
    }

    const unsubscribe = canvasSyncStream.subscribe({ clientId, canvasId }, async (payload) => {
      if (payload.type === CanvasSyncType.HEARTBEAT) {
        // pass
      } else if (payload.type === CanvasSyncType.UPDATE) {
        Y.applyUpdateV2(doc, base64.parse(payload.data), 'remote');
      } else if (payload.type === CanvasSyncType.VECTOR) {
        const update = Y.encodeStateAsUpdateV2(doc, base64.parse(payload.data));

        await syncCanvas(
          {
            clientId,
            canvasId,
            type: CanvasSyncType.UPDATE,
            data: base64.stringify(update),
          },
          { transport: 'ws' },
        );
      } else if (payload.type === CanvasSyncType.AWARENESS) {
        YAwareness.applyAwarenessUpdate(awareness, base64.parse(payload.data), 'remote');
      } else if (payload.type === CanvasSyncType.PRESENCE) {
        const update = YAwareness.encodeAwarenessUpdate(awareness, [doc.clientID]);

        await syncCanvas(
          {
            clientId,
            canvasId,
            type: CanvasSyncType.AWARENESS,
            data: base64.stringify(update),
          },
          { transport: 'ws' },
        );
      }
    });

    const persistence = new IndexeddbPersistence(`typie:canvas:${canvasId}`, doc);
    persistence.on('synced', () => forceSync());

    awareness.setLocalStateField('user', {
      name: 'Anonymous',
      color: random({ luminosity: 'bright', seed: stringHash('anonymous') }).toHexString(),
    });

    canvas = new Canvas(container, doc, awareness);

    const forceSyncInterval = setInterval(() => forceSync(), 10_000);

    return () => {
      clearInterval(forceSyncInterval);

      YAwareness.removeAwarenessStates(awareness, [doc.clientID], 'local');
      unsubscribe();

      persistence.destroy();
      awareness.destroy();
      doc.destroy();

      canvas?.destroy();
    };
  });
</script>

<svelte:window on:keydown={(e) => canvas?.handleKeyDown(e)} />

<div
  class={css({
    position: 'relative',
    width: 'full',
    height: '[100dvh]',
    overflow: 'hidden',
    backgroundColor: 'surface.subtle',
  })}
>
  <div style:cursor class={css({ size: 'full', backgroundColor: 'surface.subtle' })}>
    <div bind:this={container} class={css({ size: 'full' })}></div>
  </div>

  {#if canvas}
    <Toolbar {canvas} />
    <Zoom {canvas} />
    <Panel {canvas} />
  {/if}
</div>
