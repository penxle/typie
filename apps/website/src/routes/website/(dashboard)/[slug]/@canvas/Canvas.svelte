<script lang="ts">
  import { random } from '@ctrl/tinycolor';
  import stringHash from '@sindresorhus/string-hash';
  import { nanoid } from 'nanoid';
  import { base64 } from 'rfc4648';
  import { onMount } from 'svelte';
  import { IndexeddbPersistence } from 'y-indexeddb';
  import * as YAwareness from 'y-protocols/awareness';
  import * as Y from 'yjs';
  import { CanvasSyncType } from '@/enums';
  import { browser } from '$app/environment';
  import { fragment, graphql } from '$graphql';
  import { Canvas, CanvasEditor } from '$lib/canvas';
  import { getAppContext, getThemeContext } from '$lib/context';
  import { css } from '$styled-system/css';
  import Panel from './Panel.svelte';
  import Toolbar from './Toolbar.svelte';
  import Zoom from './Zoom.svelte';
  import type { Canvas_query } from '$graphql';

  type Props = {
    $query: Canvas_query;
  };

  let { $query: _query }: Props = $props();

  const query = fragment(
    _query,
    graphql(`
      fragment Canvas_query on Query {
        me @required {
          id
          name
          role
        }

        entity(slug: $slug) {
          id
          slug
          url

          ancestors {
            id

            node {
              __typename

              ... on Folder {
                id
                name
              }
            }
          }

          node {
            __typename

            ... on Canvas {
              id
              title
              update
            }
          }
        }
      }
    `),
  );

  const syncCanvas = graphql(`
    mutation DashboardSlugPage_Canvas_SyncCanvas_Mutation($input: SyncCanvasInput!) {
      syncCanvas(input: $input)
    }
  `);

  const canvasSyncStream = graphql(`
    subscription DashboardSlugPage_Canvas_CanvasSyncStream_Subscription($clientId: String!, $canvasId: ID!) {
      canvasSyncStream(clientId: $clientId, canvasId: $canvasId) {
        canvasId
        type
        data
      }
    }
  `);

  const app = getAppContext();
  const clientId = nanoid();
  const canvasId = $derived($query.entity.node.__typename === 'Canvas' ? $query.entity.node.id : null);

  let canvas = $state<Canvas>();

  const doc = new Y.Doc();
  const awareness = new YAwareness.Awareness(doc);

  const theme = getThemeContext();
  theme.force('light');

  doc.on('updateV2', async (update, origin) => {
    if (browser && origin !== 'remote' && canvasId) {
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
    if (browser && origin !== 'remote' && canvasId) {
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
    if (!canvasId) return;

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
    if (!canvasId) return;

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

    if ($query.entity.node.__typename === 'Canvas') {
      Y.applyUpdateV2(doc, base64.parse($query.entity.node.update), 'remote');
    }

    awareness.setLocalStateField('user', {
      name: $query.me.name,
      color: random({ luminosity: 'bright', seed: stringHash($query.me.id) }).toHexString(),
    });

    const forceSyncInterval = setInterval(() => forceSync(), 10_000);

    app.state.ancestors = $query.entity.ancestors.map((ancestor) => ancestor.id);
    app.state.current = $query.entity.id;

    return () => {
      clearInterval(forceSyncInterval);

      YAwareness.removeAwarenessStates(awareness, [doc.clientID], 'local');
      unsubscribe();

      persistence.destroy();
      awareness.destroy();
      doc.destroy();
    };
  });
</script>

<div
  class={css({
    position: 'relative',
    width: 'full',
    height: '[100dvh]',
    overflow: 'hidden',
    backgroundColor: 'surface.subtle',
  })}
>
  <CanvasEditor style={css.raw({ size: 'full' })} {awareness} {doc} bind:canvas />

  {#if canvas}
    <Toolbar {canvas} />
    <Zoom {canvas} />
    <Panel {canvas} />
  {/if}
</div>
