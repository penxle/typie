<script lang="ts">
  import { random } from '@ctrl/tinycolor';
  import stringHash from '@sindresorhus/string-hash';
  import { css } from '@typie/styled-system/css';
  import { center } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { Canvas, CanvasEditor } from '@typie/ui/canvas';
  import { Helmet, Icon, Menu } from '@typie/ui/components';
  import { getAppContext, getThemeContext } from '@typie/ui/context';
  import dayjs from 'dayjs';
  import { nanoid } from 'nanoid';
  import { base64 } from 'rfc4648';
  import { onMount, tick } from 'svelte';
  import { match } from 'ts-pattern';
  import { IndexeddbPersistence } from 'y-indexeddb';
  import * as YAwareness from 'y-protocols/awareness';
  import * as Y from 'yjs';
  import { CanvasSyncType } from '@/enums';
  import ElipsisIcon from '~icons/lucide/ellipsis';
  import LineSquiggleIcon from '~icons/lucide/line-squiggle';
  import { browser } from '$app/environment';
  import { fragment, graphql } from '$graphql';
  import CanvasMenu from '../../@context-menu/CanvasMenu.svelte';
  import { YState } from '../state.svelte';
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
  const theme = getThemeContext();
  const clientId = nanoid();
  const canvasId = $derived($query && $query.entity.node.__typename === 'Canvas' ? $query.entity.node.id : null);

  let canvas = $state<Canvas>();

  let connectionStatus = $state<'connecting' | 'connected' | 'disconnected'>('connecting');
  let lastHeartbeatAt = $state(dayjs());

  const doc = new Y.Doc();
  const awareness = new YAwareness.Awareness(doc);

  const title = new YState<string>(doc, 'title', '');
  const effectiveTitle = $derived(title.current || '(제목 없음)');

  let titleInputEl = $state<HTMLInputElement>();
  let titleEditing = $state(false);
  let titleEditingText = $state('');

  let syncUpdateTimeout: NodeJS.Timeout | null = null;
  let pendingUpdate: Uint8Array | null = null;
  let lastSyncTime = Date.now();

  doc.on('updateV2', async (update, origin) => {
    if (browser && origin !== 'remote' && canvasId) {
      if (pendingUpdate) {
        pendingUpdate = Y.mergeUpdatesV2([pendingUpdate, update]);
      } else {
        pendingUpdate = update;
      }

      if (syncUpdateTimeout) {
        clearTimeout(syncUpdateTimeout);
      }

      const timeSinceLastSync = Date.now() - lastSyncTime;
      const shouldForceSync = timeSinceLastSync >= 100;

      if (shouldForceSync && pendingUpdate) {
        await syncCanvas(
          {
            clientId,
            canvasId,
            type: CanvasSyncType.UPDATE,
            data: base64.stringify(pendingUpdate),
          },
          { transport: 'ws' },
        );

        pendingUpdate = null;
        lastSyncTime = Date.now();
      } else {
        const remainingTime = Math.max(0, 100 - timeSinceLastSync);

        syncUpdateTimeout = setTimeout(async () => {
          if (pendingUpdate && canvasId) {
            await syncCanvas(
              {
                clientId,
                canvasId,
                type: CanvasSyncType.UPDATE,
                data: base64.stringify(pendingUpdate),
              },
              { transport: 'ws' },
            );

            pendingUpdate = null;
            lastSyncTime = Date.now();
          }
        }, remainingTime);
      }
    }
  });

  let syncAwarenessTimeout: NodeJS.Timeout | null = null;
  let pendingAwarenessStates: { added: number[]; updated: number[]; removed: number[] } | null = null;
  let lastAwarenessSyncTime = Date.now();

  awareness.on('update', async (states: { added: number[]; updated: number[]; removed: number[] }, origin: unknown) => {
    if (browser && origin !== 'remote' && canvasId) {
      if (pendingAwarenessStates) {
        pendingAwarenessStates = {
          added: [...new Set([...pendingAwarenessStates.added, ...states.added])],
          updated: [...new Set([...pendingAwarenessStates.updated, ...states.updated])],
          removed: [...new Set([...pendingAwarenessStates.removed, ...states.removed])],
        };
      } else {
        pendingAwarenessStates = states;
      }

      if (syncAwarenessTimeout) {
        clearTimeout(syncAwarenessTimeout);
      }

      const timeSinceLastSync = Date.now() - lastAwarenessSyncTime;
      const shouldForceSync = timeSinceLastSync >= 100;

      if (shouldForceSync && pendingAwarenessStates) {
        const update = YAwareness.encodeAwarenessUpdate(awareness, [
          ...pendingAwarenessStates.added,
          ...pendingAwarenessStates.updated,
          ...pendingAwarenessStates.removed,
        ]);

        await syncCanvas(
          {
            clientId,
            canvasId,
            type: CanvasSyncType.AWARENESS,
            data: base64.stringify(update),
          },
          { transport: 'ws' },
        );

        pendingAwarenessStates = null;
        lastAwarenessSyncTime = Date.now();
      } else {
        const remainingTime = Math.max(0, 100 - timeSinceLastSync);

        syncAwarenessTimeout = setTimeout(async () => {
          if (pendingAwarenessStates && canvasId) {
            const update = YAwareness.encodeAwarenessUpdate(awareness, [
              ...pendingAwarenessStates.added,
              ...pendingAwarenessStates.updated,
              ...pendingAwarenessStates.removed,
            ]);

            await syncCanvas(
              {
                clientId,
                canvasId,
                type: CanvasSyncType.AWARENESS,
                data: base64.stringify(update),
              },
              { transport: 'ws' },
            );

            pendingAwarenessStates = null;
            lastAwarenessSyncTime = Date.now();
          }
        }, remainingTime);
      }
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
        lastHeartbeatAt = dayjs(payload.data);
        connectionStatus = 'connected';
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
    const heartbeatInterval = setInterval(() => {
      if (dayjs().diff(lastHeartbeatAt, 'seconds') > 10) {
        connectionStatus = 'disconnected';
      }
    }, 1000);

    app.state.ancestors = $query.entity.ancestors.map((ancestor) => ancestor.id);
    app.state.current = $query.entity.id;

    if (canvas) {
      const { x, y, width, height } = canvas.scene.getLayer().getClientRect();
      const stageWidth = canvas.stage.width();
      const stageHeight = canvas.stage.height();

      canvas.moveTo(-(x + width / 2 - stageWidth / 2), -(y + height / 2 - stageHeight / 2));

      // 여유도 주고 node 없을 때 div0 되지 않게 100 더함
      canvas.scaleTo(Math.min(stageWidth / (width + 100), stageHeight / (height + 100), 1));
    }

    return () => {
      clearInterval(forceSyncInterval);
      clearInterval(heartbeatInterval);

      if (syncUpdateTimeout) {
        clearTimeout(syncUpdateTimeout);
      }

      if (syncAwarenessTimeout) {
        clearTimeout(syncAwarenessTimeout);
      }

      YAwareness.removeAwarenessStates(awareness, [doc.clientID], 'local');
      unsubscribe();

      persistence.destroy();
      awareness.destroy();
      doc.destroy();
    };
  });

  $effect(() => {
    const currentTheme = theme.effective;

    if (canvas && currentTheme) {
      canvas.environment.update();
      canvas.stage.batchDraw();
    }
  });
</script>

<Helmet title={`${effectiveTitle} 그리는 중`} />

<div class={css({ position: 'relative', size: 'full', overflow: 'hidden' })}>
  <CanvasEditor style={css.raw({ size: 'full' })} {awareness} {doc} bind:canvas />

  <div
    class={center({
      position: 'absolute',
      top: '20px',
      left: '20px',
      gap: '12px',
      borderRadius: '12px',
      paddingX: '16px',
      paddingY: '12px',
      color: 'text.default',
      backgroundColor: 'surface.default',
      boxShadow: 'small',
    })}
  >
    <Icon style={css.raw({ color: 'text.faint' })} icon={LineSquiggleIcon} size={16} />

    <div class={css({ width: '1px', height: '16px', backgroundColor: 'border.default' })}></div>

    <div
      style:background-color={match(connectionStatus)
        .with('connecting', () => '#eab308')
        .with('connected', () => '#22c55e')
        .with('disconnected', () => '#ef4444')
        .exhaustive()}
      class={css({ size: '8px', borderRadius: 'full' })}
      use:tooltip={{
        message: match(connectionStatus)
          .with('connecting', () => '서버 연결 중...')
          .with('connected', () => '실시간 저장 중')
          .with('disconnected', () => '서버 연결 끊김')
          .exhaustive(),
        placement: 'left',
        offset: 12,
        delay: 0,
      }}
    ></div>

    {#if titleEditing}
      <input
        bind:this={titleInputEl}
        class={css({ fontSize: '14px', fontWeight: 'bold' })}
        onblur={() => {
          titleEditing = false;
          title.current = titleEditingText;
        }}
        onkeydown={(e) => {
          e.stopPropagation();

          if (e.key === 'Enter') {
            titleEditing = false;
            title.current = titleEditingText;
          } else if (e.key === 'Escape') {
            titleEditing = false;
            titleEditingText = title.current;
          }
        }}
        placeholder="(제목 없음)"
        type="text"
        bind:value={titleEditingText}
      />
    {:else}
      <button
        class={css({ fontSize: '14px', fontWeight: 'bold', cursor: 'text' })}
        ondblclick={async () => {
          titleEditingText = title.current;
          titleEditing = true;
          await tick();
          titleInputEl?.select();
        }}
        type="button"
      >
        {effectiveTitle}
      </button>
    {/if}

    <Menu placement="bottom-start">
      {#snippet button({ open })}
        <button
          class={center({
            borderRadius: '4px',
            size: '24px',
            color: 'text.faint',
            transition: 'common',
            _hover: {
              color: 'text.subtle',
              backgroundColor: 'surface.muted',
            },
            _pressed: {
              color: 'text.subtle',
              backgroundColor: 'surface.muted',
            },
          })}
          aria-pressed={open}
          type="button"
        >
          <Icon icon={ElipsisIcon} size={16} />
        </button>
      {/snippet}

      {#if $query.entity.node.__typename === 'Canvas'}
        <CanvasMenu canvas={$query.entity.node} via="editor" />
      {/if}
    </Menu>
  </div>

  {#if canvas}
    <Toolbar {canvas} />
    <Zoom {canvas} />
    <Panel {canvas} />
  {/if}
</div>
