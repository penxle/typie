<script lang="ts">
  import { random } from '@ctrl/tinycolor';
  import stringHash from '@sindresorhus/string-hash';
  import dayjs from 'dayjs';
  import { nanoid } from 'nanoid';
  import { base64 } from 'rfc4648';
  import { onMount } from 'svelte';
  import { on } from 'svelte/events';
  import { match } from 'ts-pattern';
  import { IndexeddbPersistence } from 'y-indexeddb';
  import * as YAwareness from 'y-protocols/awareness';
  import * as Y from 'yjs';
  import { PostSyncType } from '@/enums';
  import BlendIcon from '~icons/lucide/blend';
  import ChevronRightIcon from '~icons/lucide/chevron-right';
  import CopyIcon from '~icons/lucide/copy';
  import ElipsisIcon from '~icons/lucide/ellipsis';
  import LibraryBigIcon from '~icons/lucide/library-big';
  import PanelRightCloseIcon from '~icons/lucide/panel-right-close';
  import PanelRightOpenIcon from '~icons/lucide/panel-right-open';
  import TrashIcon from '~icons/lucide/trash';
  import { browser } from '$app/environment';
  import { goto } from '$app/navigation';
  import { fragment, graphql } from '$graphql';
  import { autosize, tooltip } from '$lib/actions';
  import { Helmet, HorizontalDivider, Icon, Menu, MenuItem } from '$lib/components';
  import { getAppContext } from '$lib/context';
  import { Dialog, Tip } from '$lib/notification';
  import { TiptapEditor } from '$lib/tiptap';
  import { css } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';
  import Panel from './Panel.svelte';
  import { YState } from './state.svelte';
  import Toolbar from './Toolbar.svelte';
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
          update

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

          ...Editor_Panel_post
        }
      }
    `),
  );

  const syncPost = graphql(`
    mutation Editor_SyncPost_Mutation($input: SyncPostInput!) {
      syncPost(input: $input)
    }
  `);

  const duplicatePost = graphql(`
    mutation Editor_DuplicatePost_Mutation($input: DuplicatePostInput!) {
      duplicatePost(input: $input) {
        id

        entity {
          id
          slug
        }
      }
    }
  `);

  const deletePost = graphql(`
    mutation Editor_DeletePost_Mutation($input: DeletePostInput!) {
      deletePost(input: $input) {
        id
      }
    }
  `);

  const postSyncStream = graphql(`
    subscription Editor_PostSyncStream_Subscription($clientId: String!, $postId: ID!) {
      postSyncStream(clientId: $clientId, postId: $postId) {
        postId
        type
        data
      }
    }
  `);

  const app = getAppContext();
  const clientId = nanoid();

  let titleEl = $state<HTMLTextAreaElement>();
  let subtitleEl = $state<HTMLTextAreaElement>();

  let editor = $state<Ref<Editor>>();

  let connectionStatus = $state<'connecting' | 'connected' | 'disconnected'>('connecting');
  let lastHeartbeatAt = $state(dayjs());

  const doc = new Y.Doc();
  const awareness = new YAwareness.Awareness(doc);

  const title = new YState<string>(doc, 'title', '');
  const subtitle = new YState<string>(doc, 'subtitle', '');
  const maxWidth = new YState<number>(doc, 'maxWidth', 800);

  const effectiveTitle = $derived(title.current || '(제목 없음)');

  doc.on('updateV2', async (update, origin) => {
    if (browser && origin !== 'remote') {
      await syncPost(
        {
          clientId,
          postId: $query.post.id,
          type: PostSyncType.UPDATE,
          data: base64.stringify(update),
        },
        { transport: 'ws' },
      );
    }
  });

  awareness.on('update', async (states: { added: number[]; updated: number[]; removed: number[] }, origin: unknown) => {
    if (browser && origin !== 'remote') {
      const update = YAwareness.encodeAwarenessUpdate(awareness, [...states.added, ...states.updated, ...states.removed]);

      await syncPost(
        {
          clientId,
          postId: $query.post.id,
          type: PostSyncType.AWARENESS,
          data: base64.stringify(update),
        },
        { transport: 'ws' },
      );
    }
  });

  const forceSync = async () => {
    const vector = Y.encodeStateVector(doc);

    await syncPost(
      {
        clientId,
        postId: $query.post.id,
        type: PostSyncType.VECTOR,
        data: base64.stringify(vector),
      },
      { transport: 'ws' },
    );
  };

  onMount(() => {
    const unsubscribe = postSyncStream.subscribe({ clientId, postId: $query.post.id }, async (payload) => {
      if (payload.type === PostSyncType.HEARTBEAT) {
        lastHeartbeatAt = dayjs(payload.data);
        connectionStatus = 'connected';
      } else if (payload.type === PostSyncType.UPDATE) {
        Y.applyUpdateV2(doc, base64.parse(payload.data), 'remote');
      } else if (payload.type === PostSyncType.VECTOR) {
        const update = Y.encodeStateAsUpdateV2(doc, base64.parse(payload.data));

        await syncPost(
          {
            clientId,
            postId: $query.post.id,
            type: PostSyncType.UPDATE,
            data: base64.stringify(update),
          },
          { transport: 'ws' },
        );
      } else if (payload.type === PostSyncType.AWARENESS) {
        YAwareness.applyAwarenessUpdate(awareness, base64.parse(payload.data), 'remote');
      } else if (payload.type === PostSyncType.PRESENCE) {
        const update = YAwareness.encodeAwarenessUpdate(awareness, [doc.clientID]);

        await syncPost(
          {
            clientId,
            postId: $query.post.id,
            type: PostSyncType.AWARENESS,
            data: base64.stringify(update),
          },
          { transport: 'ws' },
        );
      }
    });

    const persistence = new IndexeddbPersistence(`typie:editor:${$query.post.id}`, doc);
    persistence.on('synced', () => forceSync());

    Y.applyUpdateV2(doc, base64.parse($query.post.update), 'remote');
    awareness.setLocalStateField('user', {
      name: $query.me.name,
      color: random({ luminosity: 'bright', seed: stringHash($query.me.id) }).toHexString(),
    });

    editor?.current.commands.setTextSelection(0);

    const forceSyncInterval = setInterval(() => forceSync(), 10_000);
    const heartbeatInterval = setInterval(() => {
      if (dayjs().diff(lastHeartbeatAt, 'seconds') > 10) {
        connectionStatus = 'disconnected';
      }
    }, 1000);

    const off = on(globalThis.window, 'keydown', (e) => {
      if ((e.metaKey || e.ctrlKey) && e.key === 's') {
        e.preventDefault();
        e.stopPropagation();

        forceSync();
        Tip.show('editor.shortcut.save', '따로 저장 키를 누르지 않아도 모든 변경 사항은 실시간으로 저장돼요.');
      }
    });

    app.state.ancestors = $query.post.entity.ancestors.map((ancestor) => ancestor.id);
    app.state.current = $query.post.entity.id;

    return () => {
      off();

      clearInterval(forceSyncInterval);
      clearInterval(heartbeatInterval);

      YAwareness.removeAwarenessStates(awareness, [doc.clientID], 'local');
      unsubscribe();

      persistence.destroy();
      awareness.destroy();
      doc.destroy();
    };
  });
</script>

<Helmet title={`${effectiveTitle} 작성 중`} />

<div class={flex({ height: 'full' })}>
  <div class={flex({ flexDirection: 'column', flexGrow: '1' })}>
    <div
      class={flex({
        justifyContent: 'space-between',
        alignItems: 'center',
        gap: '6px',
        flexShrink: '0',
        paddingLeft: '24px',
        paddingRight: '8px',
        height: '36px',
      })}
    >
      <div class={flex({ alignItems: 'center', gap: '4px' })}>
        <Icon style={css.raw({ color: 'gray.400' })} icon={LibraryBigIcon} size={12} />

        <div class={css({ flex: 'none', fontSize: '12px', color: 'gray.400' })}>내 스페이스</div>
        <Icon style={css.raw({ color: 'gray.400' })} icon={ChevronRightIcon} size={12} />

        {#each $query.post.entity.ancestors as ancestor (ancestor.id)}
          {#if ancestor.node.__typename === 'Folder'}
            <div class={css({ fontSize: '12px', color: 'gray.400' })}>{ancestor.node.name}</div>
            <Icon style={css.raw({ color: 'gray.400' })} icon={ChevronRightIcon} size={12} />
          {/if}
        {/each}

        <div class={css({ flex: 'none', fontSize: '12px', fontWeight: 'medium', color: 'gray.700' })}>{effectiveTitle}</div>
      </div>

      <div class={flex({ alignItems: 'center', gap: '4px' })}>
        <div class={center({ size: '24px' })}>
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
        </div>

        <Menu>
          {#snippet button({ open })}
            <button
              class={center({
                borderRadius: '4px',
                size: '24px',
                color: 'gray.500',
                transition: 'common',
                _hover: {
                  color: 'gray.700',
                  backgroundColor: 'gray.100',
                },
                _pressed: {
                  color: 'gray.700',
                  backgroundColor: 'gray.100',
                },
              })}
              aria-pressed={open}
              type="button"
            >
              <Icon icon={ElipsisIcon} size={16} />
            </button>
          {/snippet}

          <MenuItem icon={BlendIcon} onclick={() => (app.state.shareOpen = $query.post.entity.id)}>공유</MenuItem>

          <MenuItem
            icon={CopyIcon}
            onclick={async () => {
              const resp = await duplicatePost({ postId: $query.post.id });
              await goto(`/${resp.entity.slug}`);
            }}
          >
            복제
          </MenuItem>

          <HorizontalDivider color="secondary" />

          <MenuItem
            icon={TrashIcon}
            onclick={() => {
              Dialog.confirm({
                title: '포스트 삭제',
                message: '정말 이 포스트를 삭제하시겠어요?',
                action: 'danger',
                actionLabel: '삭제',
                actionHandler: async () => {
                  await deletePost({ postId: $query.post.id });
                  app.state.ancestors = [];
                  app.state.current = undefined;
                },
              });
            }}
            variant="danger"
          >
            삭제
          </MenuItem>
        </Menu>

        <button
          class={center({
            borderRadius: '4px',
            size: '24px',
            color: 'gray.500',
            transition: 'common',
            _hover: { backgroundColor: 'gray.100' },
          })}
          onclick={() => (app.preference.current.panelExpanded = !app.preference.current.panelExpanded)}
          type="button"
          use:tooltip={{ message: app.preference.current.panelExpanded ? '패널 닫기' : '패널 열기' }}
        >
          <Icon
            style={css.raw({ color: 'gray.500' })}
            icon={app.preference.current.panelExpanded ? PanelRightCloseIcon : PanelRightOpenIcon}
            size={16}
          />
        </button>
      </div>
    </div>

    <HorizontalDivider color="secondary" />

    <Toolbar {doc} {editor} />

    <div class={css({ position: 'relative', flexGrow: '1', overflowY: 'auto', scrollbarGutter: 'stable' })}>
      <div
        style:--prosemirror-max-width={`${maxWidth.current}px`}
        class={flex({
          flexDirection: 'column',
          alignItems: 'center',
          flexGrow: '1',
          paddingTop: '60px',
          paddingX: '80px',
          width: 'full',
        })}
      >
        <div class={flex({ flexDirection: 'column', width: 'full', maxWidth: 'var(--prosemirror-max-width)' })}>
          <textarea
            bind:this={titleEl}
            class={css({ width: 'full', fontSize: '28px', fontWeight: 'bold', resize: 'none' })}
            autocapitalize="off"
            autocomplete="off"
            maxlength="100"
            onkeydown={(e) => {
              if (e.isComposing) {
                return;
              }

              if (e.key === 'Enter' || e.key === 'ArrowDown') {
                e.preventDefault();
                subtitleEl?.focus();
              }
            }}
            placeholder="제목을 입력하세요"
            rows={1}
            spellcheck="false"
            bind:value={title.current}
            use:autosize
          ></textarea>

          <textarea
            bind:this={subtitleEl}
            class={css({ marginTop: '4px', width: 'full', fontSize: '16px', fontWeight: 'medium', overflow: 'hidden', resize: 'none' })}
            autocapitalize="off"
            autocomplete="off"
            maxlength="100"
            onkeydown={(e) => {
              if (e.isComposing) {
                return;
              }

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
            spellcheck="false"
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

  <Panel $post={$query.post} {editor} />
</div>
