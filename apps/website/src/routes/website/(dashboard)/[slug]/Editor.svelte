<script lang="ts">
  import { random } from '@ctrl/tinycolor';
  import stringHash from '@sindresorhus/string-hash';
  import dayjs from 'dayjs';
  import { base64 } from 'rfc4648';
  import { onMount } from 'svelte';
  import { IndexeddbPersistence } from 'y-indexeddb';
  import * as YAwareness from 'y-protocols/awareness';
  import * as Y from 'yjs';
  import { PostContentSyncMessageKind } from '@/const';
  import { browser } from '$app/environment';
  import { fragment, graphql } from '$graphql';
  import { autosize } from '$lib/actions';
  import { Button, Helmet, HorizontalDivider } from '$lib/components';
  import { TiptapEditor } from '$lib/tiptap';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
  import TopBar from '../TopBar.svelte';
  import { YState } from './state.svelte';
  import StatusBar from './StatusBar.svelte';
  import Toolbar from './Toolbar.svelte';
  import { createWebSocket } from './ws.svelte';
  import type { Editor } from '@tiptap/core';
  import type { Editor_query } from '$graphql';
  import type { Ref } from '$lib/utils';

  type Props = {
    $query: Editor_query;
  };

  let { $query: _query }: Props = $props();

  const query = fragment(
    _query,
    graphql(`
      fragment Editor_query on Query {
        me @required {
          id
          name
        }

        post(slug: $slug) {
          id

          entity {
            id
            slug

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

            site {
              id
              url
            }
          }

          content {
            id
            update
          }
        }
      }
    `),
  );

  let titleEl = $state<HTMLTextAreaElement>();
  let subtitleEl = $state<HTMLTextAreaElement>();

  let editor = $state<Ref<Editor>>();

  let connectionStatus = $state<'connecting' | 'connected' | 'disconnected'>('connecting');
  let lastHeartbeatAt = $state(dayjs());

  const doc = new Y.Doc();
  const awareness = new YAwareness.Awareness(doc);

  const title = new YState<string>(doc, 'title', '');
  const subtitle = new YState<string>(doc, 'subtitle', '');
  const maxWidth = new YState<number>(doc, 'maxWidth', 1000);

  const effectiveTitle = $derived(title.current || '(제목 없음)');

  const encoder = new TextEncoder();

  const ws = createWebSocket({
    onopen: () => {
      connectionStatus = 'connecting';
      ws.send(PostContentSyncMessageKind.INIT, encoder.encode($query.post.id));
    },
    onmessage: (type, data) => {
      if (type === PostContentSyncMessageKind.HEARTBEAT) {
        lastHeartbeatAt = dayjs();
      } else if (type === PostContentSyncMessageKind.INIT) {
        connectionStatus = 'connected';
        forceSync();
      } else if (type === PostContentSyncMessageKind.UPDATE) {
        Y.applyUpdateV2(doc, data, 'remote');
      } else if (type === PostContentSyncMessageKind.VECTOR) {
        const update = Y.encodeStateAsUpdateV2(doc, data);
        ws.send(PostContentSyncMessageKind.UPDATE, update);
      } else if (type === PostContentSyncMessageKind.AWARENESS) {
        YAwareness.applyAwarenessUpdate(awareness, data, 'remote');
      } else if (type === PostContentSyncMessageKind.PRESENCE) {
        const update = YAwareness.encodeAwarenessUpdate(awareness, [doc.clientID]);
        ws.send(PostContentSyncMessageKind.AWARENESS, update);
      }
    },
    onclose: () => {
      connectionStatus = 'disconnected';
    },
  });

  doc.on('updateV2', async (update, origin) => {
    if (browser && origin !== 'remote') {
      ws.send(PostContentSyncMessageKind.UPDATE, update);
    }
  });

  awareness.on('update', async (states: { added: number[]; updated: number[]; removed: number[] }, origin: unknown) => {
    if (browser && origin !== 'remote') {
      const update = YAwareness.encodeAwarenessUpdate(awareness, [...states.added, ...states.updated, ...states.removed]);
      ws.send(PostContentSyncMessageKind.AWARENESS, update);
    }
  });

  const forceSync = async () => {
    const vector = Y.encodeStateVector(doc);
    ws.send(PostContentSyncMessageKind.VECTOR, vector);
  };

  onMount(() => {
    const persistence = new IndexeddbPersistence(`typie:editor:${$query.post.id}`, doc);

    Y.applyUpdateV2(doc, base64.parse($query.post.content.update), 'remote');
    awareness.setLocalStateField('user', {
      name: $query.me.name,
      color: random({ luminosity: 'bright', seed: stringHash($query.me.id) }).toHexString(),
    });

    editor?.current.commands.setTextSelection(0);

    const forceSyncInterval = setInterval(() => forceSync(), 10_000);
    const heartbeatInterval = setInterval(() => {
      if (dayjs().diff(lastHeartbeatAt, 'seconds') > 10) {
        ws.reconnect();
      }
    }, 1000);

    return () => {
      clearInterval(forceSyncInterval);
      clearInterval(heartbeatInterval);

      YAwareness.removeAwarenessStates(awareness, [doc.clientID], 'local');

      persistence.destroy();
      awareness.destroy();
      doc.destroy();
    };
  });
</script>

<Helmet title={`${effectiveTitle} 작성 중`} />

<TopBar>
  <div class={flex({ justifyContent: 'space-between', alignItems: 'center' })}>
    <div class={flex({ alignItems: 'center', gap: '6px' })}>
      {#each $query.post.entity.ancestors as ancestor (ancestor.id)}
        {#if ancestor.node.__typename === 'Folder'}
          <div class={css({ fontSize: '14px', fontWeight: 'medium', color: 'gray.500' })}>{ancestor.node.name}</div>
          <div class={css({ fontSize: '14px', fontWeight: 'medium', color: 'gray.200' })}>/</div>
        {/if}
      {/each}

      <div class={css({ fontSize: '14px', fontWeight: 'medium' })}>{effectiveTitle}</div>
    </div>

    <div>
      <Button external href={`${$query.post.entity.site.url}/${$query.post.entity.slug}`} size="sm" type="link" variant="secondary">
        공유
      </Button>
    </div>
  </div>
</TopBar>

<HorizontalDivider />

<div class={flex({ flexDirection: 'column', alignItems: 'center', flexGrow: '1', overflow: 'hidden' })}>
  <Toolbar {doc} {editor} />

  <div
    class={flex({
      flexDirection: 'column',
      alignItems: 'center',
      flexGrow: '1',
      width: 'full',
      backgroundColor: 'gray.100',
      overflow: 'scroll',
    })}
  >
    <div
      style:--prosemirror-max-width={`${maxWidth.current}px`}
      class={flex({
        flexDirection: 'column',
        alignItems: 'center',
        flexGrow: '1',
        paddingTop: '40px',
        width: 'full',
        maxWidth: '1200px',
        backgroundColor: 'white',
      })}
    >
      <div class={flex({ flexDirection: 'column', width: 'full', maxWidth: 'var(--prosemirror-max-width)' })}>
        <textarea
          bind:this={titleEl}
          class={css({ width: 'full', fontSize: '28px', fontWeight: 'bold', resize: 'none' })}
          maxlength="100"
          onkeydown={(e) => {
            if (e.key === 'Enter' || e.key === 'ArrowDown') {
              e.preventDefault();
              subtitleEl?.focus();
            }
          }}
          placeholder="제목을 입력하세요"
          rows={1}
          bind:value={title.current}
          use:autosize
        ></textarea>

        <textarea
          bind:this={subtitleEl}
          class={css({ marginTop: '4px', width: 'full', fontSize: '16px', fontWeight: 'medium', resize: 'none' })}
          maxlength="100"
          onkeydown={(e) => {
            if (e.key === 'ArrowUp' || (e.key === 'Backspace' && !subtitleEl?.value)) {
              e.preventDefault();
              titleEl?.focus();
            }

            if (e.key === 'Enter' || e.key === 'ArrowDown' || (e.key === 'Tab' && !e.shiftKey)) {
              e.preventDefault();
              editor?.current.chain().focus().setTextSelection(2).run();
            }
          }}
          placeholder="부제목을 입력하세요"
          rows={1}
          bind:value={subtitle.current}
          use:autosize
        ></textarea>

        <HorizontalDivider style={css.raw({ marginTop: '10px', marginBottom: '20px' })} />
      </div>

      <TiptapEditor
        style={css.raw({ flexGrow: '1', width: 'full' })}
        {awareness}
        {doc}
        oncreate={() => {
          titleEl?.focus();
        }}
        onkeydown={(view, e) => {
          const { doc, selection } = view.state;
          const { anchor } = selection;

          if (
            ((e.key === 'ArrowUp' || (e.key === 'Tab' && e.shiftKey)) && anchor === 2) ||
            (e.key === 'Backspace' && doc.child(0).childCount === 1 && doc.child(0).child(0).childCount === 0)
          ) {
            e.preventDefault();
            subtitleEl?.focus();
          }
        }}
        bind:editor
      />
    </div>
  </div>
</div>

<HorizontalDivider />

<StatusBar {connectionStatus} {editor} />
