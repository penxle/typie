<script lang="ts">
  import { random } from '@ctrl/tinycolor';
  import stringHash from '@sindresorhus/string-hash';
  import { Mark } from '@tiptap/pm/model';
  import { Selection } from '@tiptap/pm/state';
  import dayjs from 'dayjs';
  import stringify from 'fast-json-stable-stringify';
  import mixpanel from 'mixpanel-browser';
  import { nanoid } from 'nanoid';
  import { base64 } from 'rfc4648';
  import { onMount, untrack } from 'svelte';
  import { on } from 'svelte/events';
  import { match } from 'ts-pattern';
  import { IndexeddbPersistence } from 'y-indexeddb';
  import * as YAwareness from 'y-protocols/awareness';
  import * as Y from 'yjs';
  import { PostSyncType, PostType, UserRole } from '@/enums';
  import BlendIcon from '~icons/lucide/blend';
  import ChevronRightIcon from '~icons/lucide/chevron-right';
  import IconClockFading from '~icons/lucide/clock-fading';
  import CopyIcon from '~icons/lucide/copy';
  import ElipsisIcon from '~icons/lucide/ellipsis';
  import ExternalLinkIcon from '~icons/lucide/external-link';
  import FolderIcon from '~icons/lucide/folder';
  import Maximize2Icon from '~icons/lucide/maximize-2';
  import PanelRightCloseIcon from '~icons/lucide/panel-right-close';
  import PanelRightOpenIcon from '~icons/lucide/panel-right-open';
  import ShapesIcon from '~icons/lucide/shapes';
  import TrashIcon from '~icons/lucide/trash';
  import { browser } from '$app/environment';
  import { goto } from '$app/navigation';
  import { fragment, graphql } from '$graphql';
  import { autosize, tooltip } from '$lib/actions';
  import { Helmet, HorizontalDivider, Icon, Menu, MenuItem } from '$lib/components';
  import { getAppContext } from '$lib/context';
  import { Dialog, Tip } from '$lib/notification';
  import { getNodeView, TiptapEditor } from '$lib/tiptap';
  import { uploadBlobAsFile, uploadBlobAsImage } from '$lib/utils';
  import { css, cx } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';
  import Anchor from './Anchor.svelte';
  import Highlight from './Highlight.svelte';
  import Limit from './Limit.svelte';
  import Panel from './Panel.svelte';
  import PanelNote from './PanelNote.svelte';
  import Placeholder from './Placeholder.svelte';
  import { YState } from './state.svelte';
  import Timeline from './Timeline.svelte';
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
        ...Editor_Limit_query

        me @required {
          id
          name
          role

          ...Editor_Panel_user
        }

        entity(slug: $slug) {
          id
          slug
          url

          parent {
            id

            children {
              id
              slug

              node {
                __typename

                ... on Post {
                  id
                  title
                }
              }
            }
          }

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

            entities {
              id
              slug

              node {
                __typename

                ... on Post {
                  id
                  title
                }
              }
            }

            fonts {
              id
              name
              weight
              url
            }

            ...Editor_Limit_site
            ...Editor_Placeholder_site
            ...Editor_Toolbar_site
          }

          user {
            id
          }

          node {
            __typename

            ... on Post {
              id
              title
              type
              update

              ...Editor_Panel_post
              ...Editor_Timeline_post
            }
          }
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

  const updatePostType = graphql(`
    mutation Editor_UpdatePostType_Mutation($input: UpdatePostTypeInput!) {
      updatePostType(input: $input) {
        id
        type

        entity {
          id

          site {
            id

            templates {
              id
            }
          }
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
  const postId = $derived($query && $query.entity.node.__typename === 'Post' ? $query.entity.node.id : null);

  let titleEl = $state<HTMLTextAreaElement>();
  let subtitleEl = $state<HTMLTextAreaElement>();

  let editor = $state<Ref<Editor>>();

  let connectionStatus = $state<'connecting' | 'connected' | 'disconnected'>('connecting');
  let lastHeartbeatAt = $state(dayjs());

  let mounted = $state(false);

  let showTimeline = $state(false);
  let showAnchorOutline = $state(false);

  const doc = new Y.Doc();
  const awareness = new YAwareness.Awareness(doc);

  const title = new YState<string>(doc, 'title', '');
  const subtitle = new YState<string>(doc, 'subtitle', '');
  const maxWidth = new YState<number>(doc, 'maxWidth', 800);
  const storedMarks = new YState<unknown[]>(doc, 'storedMarks', []);
  const anchors = new YState<Record<string, string | null>>(doc, 'anchors', {});

  const effectiveTitle = $derived(title.current || '(제목 없음)');

  const anchorElements = $derived.by(() => {
    if (!editor) {
      return {};
    }

    const elements: Record<string, HTMLElement> = {};

    for (const nodeId of Object.keys(anchors.current)) {
      const element = document.querySelector(`[data-node-id="${nodeId}"]`);
      if (element) {
        elements[nodeId] = element as HTMLElement;
      }
    }

    return elements;
  });

  const anchorPositions = $derived.by(() => {
    if (!editor || Object.keys(anchorElements).length === 0) return [];

    const editorEl = document.querySelector('.editor');
    if (!editorEl) return [];

    const totalHeight = editorEl.scrollHeight;
    if (totalHeight === 0) return [];

    return Object.entries(anchorElements).map(([nodeId, element]) => {
      const offsetTop = element.offsetTop;
      const position = Math.min(1, Math.max(0, offsetTop / totalHeight));

      return {
        nodeId,
        element,
        position,
        name:
          anchors.current[nodeId] ||
          (element.textContent
            ? element.textContent.length > 20
              ? element.textContent.slice(0, 20) + '...'
              : element.textContent
            : '(내용 없음)'),
      };
    });
  });

  const persistSelection = () => {
    if (!editor?.current || !postId) return;

    const { selection } = editor.current.state;

    const selections = JSON.parse(localStorage.getItem('typie:selections') || '{}');
    selections[postId] = { ...selection.toJSON(), timestamp: dayjs().valueOf() };
    localStorage.setItem('typie:selections', JSON.stringify(selections));
  };

  const fontFaces = $derived(
    $query.entity.site.fonts
      .map(
        (font) =>
          `@font-face { font-family: ${font.id}; src: url(${font.url}) format('woff2'); font-weight: ${font.weight}; font-display: block; }`,
      )
      .join('\n'),
  );

  let syncUpdateTimeout: NodeJS.Timeout | null = null;
  let pendingUpdate: Uint8Array | null = null;

  doc.on('updateV2', async (update, origin) => {
    if (browser && origin !== 'remote' && postId) {
      if (pendingUpdate) {
        pendingUpdate = Y.mergeUpdatesV2([pendingUpdate, update]);
      } else {
        pendingUpdate = update;
      }

      if (syncUpdateTimeout) {
        clearTimeout(syncUpdateTimeout);
      }

      syncUpdateTimeout = setTimeout(async () => {
        if (pendingUpdate) {
          await syncPost(
            {
              clientId,
              postId,
              type: PostSyncType.UPDATE,
              data: base64.stringify(pendingUpdate),
            },
            { transport: 'ws' },
          );

          pendingUpdate = null;
        }
      }, 1000);
    }
  });

  let syncAwarenessTimeout: NodeJS.Timeout | null = null;
  let pendingAwarenessStates: { added: number[]; updated: number[]; removed: number[] } | null = null;

  awareness.on('update', async (states: { added: number[]; updated: number[]; removed: number[] }, origin: unknown) => {
    if (browser && origin !== 'remote' && postId) {
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

      syncAwarenessTimeout = setTimeout(async () => {
        if (pendingAwarenessStates) {
          const update = YAwareness.encodeAwarenessUpdate(awareness, [
            ...pendingAwarenessStates.added,
            ...pendingAwarenessStates.updated,
            ...pendingAwarenessStates.removed,
          ]);

          await syncPost(
            {
              clientId,
              postId,
              type: PostSyncType.AWARENESS,
              data: base64.stringify(update),
            },
            { transport: 'ws' },
          );

          pendingAwarenessStates = null;
        }
      }, 1000);
    }
  });

  const forceSync = async () => {
    if (!postId) return;

    const vector = Y.encodeStateVector(doc);

    await syncPost(
      {
        clientId,
        postId,
        type: PostSyncType.VECTOR,
        data: base64.stringify(vector),
      },
      { transport: 'ws' },
    );
  };

  $effect(() => {
    if (app.preference.current.typewriterEnabled && app.preference.current.typewriterPosition !== undefined) {
      untrack(() => {
        if (editor) {
          editor.current.storage.typewriter = { position: app.preference.current.typewriterPosition };
        }
      });
    } else {
      untrack(() => {
        if (editor) {
          editor.current.storage.typewriter = { position: undefined };
        }
      });
    }
  });

  $effect(() => {
    if (app.preference.current.zenModeEnabled) {
      Tip.show('editor.zen-mode.enabled', '집중 모드가 활성화되었어요. Esc 키를 눌러 빠져나올 수 있어요.');
    }
  });

  onMount(() => {
    if (!postId) return;

    const unsubscribe = postSyncStream.subscribe({ clientId, postId }, async (payload) => {
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
            postId,
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
            postId,
            type: PostSyncType.AWARENESS,
            data: base64.stringify(update),
          },
          { transport: 'ws' },
        );
      }
    });

    const persistence = new IndexeddbPersistence(`typie:editor:${postId}`, doc);
    persistence.on('synced', () => forceSync());

    if ($query.entity.node.__typename === 'Post') {
      Y.applyUpdateV2(doc, base64.parse($query.entity.node.update), 'remote');
    }

    awareness.setLocalStateField('user', {
      name: $query.me.name,
      color: random({ luminosity: 'bright', seed: stringHash($query.me.id) }).toHexString(),
    });

    if (editor) {
      editor.current.storage.anchors = anchors;
    }

    editor?.current.once('create', ({ editor }) => {
      const { tr, schema } = editor.state;
      tr.setStoredMarks(storedMarks.current.map((mark) => Mark.fromJSON(schema, mark)));
      editor.view.dispatch(tr);

      const selections = JSON.parse(localStorage.getItem('typie:selections') || '{}');
      if (postId && selections[postId]) {
        if (selections[postId].type === 'element') {
          if (selections[postId].element === 'title') {
            titleEl?.focus();
          } else if (selections[postId].element === 'subtitle') {
            subtitleEl?.focus();
          }
        } else {
          const selection = Selection.fromJSON(editor.state.doc, selections[postId]);
          editor.commands.command(({ tr, dispatch }) => {
            tr.setSelection(selection);
            dispatch?.(tr);
            return true;
          });

          document.fonts.ready.then(() => {
            editor.commands.focus();
          });
        }
      } else {
        editor.commands.setTextSelection(2);
        titleEl?.focus();
      }
    });

    const forceSyncInterval = setInterval(() => forceSync(), 10_000);
    const heartbeatInterval = setInterval(() => {
      if (dayjs().diff(lastHeartbeatAt, 'seconds') > 10) {
        connectionStatus = 'disconnected';
      }
    }, 1000);

    const off = on(globalThis.window, 'keydown', async (e) => {
      if ((e.metaKey || e.ctrlKey) && e.key === 's') {
        e.preventDefault();
        e.stopPropagation();

        forceSync();
        Tip.show('editor.shortcut.save', '따로 저장 키를 누르지 않아도 모든 변경 사항은 실시간으로 저장돼요.');
      }

      if (e.altKey && (e.key === 'ArrowUp' || e.key === 'ArrowDown')) {
        e.preventDefault();
        e.stopPropagation();

        const currentEntityId = $query.entity.id;

        let siblingEntities: { id: string; slug: string; node: { __typename: string } }[] = [];

        if ($query.entity.parent) {
          siblingEntities = $query.entity.parent.children.filter((child) => child.node.__typename === 'Post');
        } else {
          siblingEntities = $query.entity.site.entities.filter((entity) => entity.node.__typename === 'Post');
        }

        const currentIndex = siblingEntities.findIndex((entity) => entity.id === currentEntityId);
        if (currentIndex === -1) return;

        let targetIndex;
        if (e.key === 'ArrowUp') {
          targetIndex = currentIndex - 1;
          if (targetIndex < 0) targetIndex = siblingEntities.length - 1;
        } else {
          targetIndex = currentIndex + 1;
          if (targetIndex >= siblingEntities.length) targetIndex = 0;
        }

        const targetEntity = siblingEntities[targetIndex];
        if (targetEntity && targetEntity.slug) {
          await goto(`/${targetEntity.slug}`);
        }
      }
    });

    app.state.ancestors = $query.entity.ancestors.map((ancestor) => ancestor.id);
    app.state.current = $query.entity.id;

    const arrayOrNull = <T,>(array: T[] | readonly T[] | null | undefined) => (array?.length ? array : null);

    const handler = ({ editor }: { editor: Editor }) => {
      const marks =
        arrayOrNull(editor.state.storedMarks) ||
        arrayOrNull(editor.state.selection.$anchor.marks()) ||
        arrayOrNull(editor.state.selection.$anchor.parent.firstChild?.firstChild?.marks) ||
        [];

      const jsonMarks = marks.map((mark) => mark.toJSON());

      if (stringify(storedMarks.current) !== stringify(jsonMarks)) {
        storedMarks.current = jsonMarks;
      }
    };

    editor?.current.on('transaction', handler);
    editor?.current.on('selectionUpdate', persistSelection);

    return () => {
      off();

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

      editor?.current.off('transaction', handler);
      editor?.current.off('selectionUpdate', persistSelection);

      persistence.destroy();
      awareness.destroy();
      doc.destroy();
    };
  });
</script>

<svelte:head>
  <!-- eslint-disable-next-line svelte/no-at-html-tags -->
  {@html '<style type="text/css"' + `>${fontFaces}</` + 'style>'}
</svelte:head>

<Helmet title={`${effectiveTitle} 작성 중`} />

{#if $query.entity.node.__typename === 'Post'}
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
          <Icon style={css.raw({ color: 'text.disabled' })} icon={FolderIcon} size={12} />

          <div class={css({ flex: 'none', fontSize: '12px', color: 'text.disabled' })}>내 포스트</div>
          <Icon style={css.raw({ color: 'text.disabled' })} icon={ChevronRightIcon} size={12} />

          {#each $query.entity.ancestors as ancestor (ancestor.id)}
            {#if ancestor.node.__typename === 'Folder'}
              <div class={css({ fontSize: '12px', color: 'text.disabled' })}>{ancestor.node.name}</div>
              <Icon style={css.raw({ color: 'text.disabled' })} icon={ChevronRightIcon} size={12} />
            {/if}
          {/each}

          <div class={css({ flex: 'none', fontSize: '12px', fontWeight: 'medium', color: 'text.subtle' })}>{effectiveTitle}</div>
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

          {#if $query.me.id === $query.entity.user.id}
            <Menu>
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

              <MenuItem external href={$query.entity.url} icon={ExternalLinkIcon} type="link">사이트에서 열기</MenuItem>

              <HorizontalDivider color="secondary" />

              <MenuItem
                icon={BlendIcon}
                onclick={() => {
                  app.state.shareOpen = $query.entity.id;
                  mixpanel.track('open_post_share_modal', { via: 'editor' });
                }}
              >
                공유 및 게시
              </MenuItem>

              <MenuItem
                icon={CopyIcon}
                onclick={async () => {
                  if ($query.entity.node.__typename === 'Post') {
                    const postId = $query.entity.node.id;

                    const resp = await duplicatePost({ postId });
                    mixpanel.track('duplicate_post', { via: 'editor' });
                    await goto(`/${resp.entity.slug}`);
                  }
                }}
              >
                복제
              </MenuItem>

              {#if $query.entity.node.type === PostType.NORMAL}
                <MenuItem
                  icon={ShapesIcon}
                  onclick={() => {
                    Dialog.confirm({
                      title: '템플릿으로 전환',
                      message:
                        '이 포스트를 템플릿으로 전환하시겠어요?\n앞으로 새 포스트를 생성할 때 이 포스트의 서식을 쉽게 이용할 수 있어요.',
                      actionLabel: '전환',
                      actionHandler: async () => {
                        if ($query.entity.node.__typename === 'Post') {
                          const postId = $query.entity.node.id;

                          await updatePostType({ postId, type: PostType.TEMPLATE });
                        }
                      },
                    });
                  }}
                >
                  템플릿으로 전환
                </MenuItem>
              {:else if $query.entity.node.type === PostType.TEMPLATE}
                <MenuItem
                  icon={ShapesIcon}
                  onclick={() => {
                    Dialog.confirm({
                      title: '포스트로 전환',
                      message: '이 템플릿을 다시 일반 포스트로 전환하시겠어요?',
                      actionLabel: '전환',
                      actionHandler: async () => {
                        if ($query.entity.node.__typename === 'Post') {
                          const postId = $query.entity.node.id;

                          await updatePostType({ postId, type: PostType.NORMAL });
                        }
                      },
                    });
                  }}
                >
                  포스트로 전환
                </MenuItem>
              {/if}

              <HorizontalDivider color="secondary" />

              {#if $query.me.role === UserRole.ADMIN}
                <MenuItem
                  icon={IconClockFading}
                  onclick={() => {
                    showTimeline = !showTimeline;
                  }}
                >
                  {#if showTimeline}
                    타임라인 닫기
                  {:else}
                    타임라인
                  {/if}
                </MenuItem>
              {/if}

              <HorizontalDivider color="secondary" />

              <MenuItem
                icon={TrashIcon}
                onclick={() => {
                  if ($query.entity.node.__typename === 'Post') {
                    const postId = $query.entity.node.id;
                    const title = $query.entity.node.title;

                    Dialog.confirm({
                      title: '포스트 삭제',
                      message: `정말 "${title}" 포스트를 삭제하시겠어요?`,
                      action: 'danger',
                      actionLabel: '삭제',
                      actionHandler: async () => {
                        await deletePost({ postId });
                        mixpanel.track('delete_post', { via: 'editor' });
                        app.state.ancestors = [];
                        app.state.current = undefined;
                      },
                    });
                  }
                }}
                variant="danger"
              >
                삭제
              </MenuItem>
            </Menu>
          {/if}

          <button
            class={center({
              borderRadius: '4px',
              size: '24px',
              color: 'text.faint',
              transition: 'common',
              _hover: { backgroundColor: 'surface.muted' },
            })}
            onclick={() => {
              app.preference.current.zenModeEnabled = !app.preference.current.zenModeEnabled;
              if (app.preference.current.zenModeEnabled) {
                mixpanel.track('zen_mode_enabled', { via: 'editor' });
              } else {
                mixpanel.track('zen_mode_disabled', { via: 'editor' });
              }
            }}
            type="button"
            use:tooltip={{
              message: app.preference.current.zenModeEnabled ? '집중 모드 끄기' : '집중 모드 켜기',
              keys: ['Mod', 'Shift', 'M'],
            }}
          >
            <Icon style={css.raw({ color: 'text.faint' })} icon={Maximize2Icon} size={16} />
          </button>

          <button
            class={center({
              borderRadius: '4px',
              size: '24px',
              color: 'text.faint',
              transition: 'common',
              _hover: { backgroundColor: 'surface.muted' },
            })}
            onclick={() => {
              app.preference.current.panelExpanded = !app.preference.current.panelExpanded;
              mixpanel.track('toggle_panel_expanded', { expanded: app.preference.current.panelExpanded });
            }}
            type="button"
            use:tooltip={{
              message: app.preference.current.panelExpanded ? '패널 닫기' : '패널 열기',
              keys: ['Mod', 'Shift', 'P'],
            }}
          >
            <Icon
              style={css.raw({ color: 'text.faint' })}
              icon={app.preference.current.panelExpanded ? PanelRightCloseIcon : PanelRightOpenIcon}
              size={16}
            />
          </button>
        </div>
      </div>

      <HorizontalDivider color="secondary" />

      <Toolbar $site={$query.entity.site} {doc} {editor} />

      <div class={flex({ position: 'relative', flexGrow: '1', overflowY: 'hidden' })}>
        <div
          style:position={app.preference.current.zenModeEnabled ? 'fixed' : 'relative'}
          style:top={app.preference.current.zenModeEnabled ? '0' : 'auto'}
          style:left={app.preference.current.zenModeEnabled ? '0' : 'auto'}
          style:right={app.preference.current.zenModeEnabled ? '0' : 'auto'}
          style:bottom={app.preference.current.zenModeEnabled ? '0' : 'auto'}
          class={flex({ position: 'relative', flexGrow: '1', zIndex: '1', backgroundColor: 'surface.default' })}
          onmouseleave={() => {
            showAnchorOutline = false;
          }}
          onmousemove={(e) => {
            const rect = e.currentTarget.getBoundingClientRect();
            const mouseX = e.clientX - rect.left;
            const width = rect.width;

            showAnchorOutline = mouseX > width - 50;
          }}
          role="none"
        >
          <div
            class={cx('editor', css({ position: 'relative', flexGrow: '1', height: 'full', overflowY: 'auto', scrollbarGutter: 'stable' }))}
          >
            <div
              style:--prosemirror-max-width={`${maxWidth.current}px`}
              style:--prosemirror-padding-bottom={`${(1 - (app.preference.current.typewriterPosition ?? 0.8)) * 100}vh`}
              class={flex({
                flexDirection: 'column',
                alignItems: 'center',
                paddingTop: '60px',
                paddingX: '80px',
                size: 'full',
              })}
            >
              <div class={flex({ flexDirection: 'column', width: 'full', maxWidth: 'var(--prosemirror-max-width)' })}>
                <textarea
                  bind:this={titleEl}
                  class={css({ width: 'full', fontSize: '28px', fontWeight: 'bold', resize: 'none' })}
                  autocapitalize="off"
                  autocomplete="off"
                  maxlength="100"
                  onfocus={() => {
                    if (postId) {
                      const selections = JSON.parse(localStorage.getItem('typie:selections') || '{}');
                      selections[postId] = { type: 'element', element: 'title', timestamp: dayjs().valueOf() };
                      localStorage.setItem('typie:selections', JSON.stringify(selections));
                    }
                  }}
                  onkeydown={(e) => {
                    if (e.isComposing) {
                      return;
                    }

                    if (e.key === 'Enter' || (!e.altKey && e.key === 'ArrowDown')) {
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
                  class={css({
                    marginTop: '4px',
                    width: 'full',
                    fontSize: '16px',
                    fontWeight: 'medium',
                    overflow: 'hidden',
                    resize: 'none',
                  })}
                  autocapitalize="off"
                  autocomplete="off"
                  maxlength="100"
                  onfocus={() => {
                    if (postId) {
                      const selections = JSON.parse(localStorage.getItem('typie:selections') || '{}');
                      selections[postId] = { type: 'element', element: 'subtitle', timestamp: dayjs().valueOf() };
                      localStorage.setItem('typie:selections', JSON.stringify(selections));
                    }
                  }}
                  onkeydown={(e) => {
                    if (e.isComposing) {
                      return;
                    }

                    if ((!e.altKey && e.key === 'ArrowUp') || (e.key === 'Backspace' && !subtitleEl?.value)) {
                      e.preventDefault();
                      titleEl?.focus();
                    }

                    if (e.key === 'Enter' || (!e.altKey && e.key === 'ArrowDown') || (e.key === 'Tab' && !e.shiftKey)) {
                      e.preventDefault();
                      const marks = editor?.current.state.storedMarks || editor?.current.state.selection.$anchor.marks() || null;
                      editor?.current
                        .chain()
                        .focus()
                        .setTextSelection(2)
                        .command(({ tr, dispatch }) => {
                          tr.setStoredMarks(marks);
                          dispatch?.(tr);
                          return true;
                        })
                        .run();
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

              <div class={css({ position: 'relative', flexGrow: '1', width: 'full', zIndex: '0' })}>
                <TiptapEditor
                  style={css.raw({ size: 'full' })}
                  {awareness}
                  {doc}
                  oncreate={() => {
                    mounted = true;
                  }}
                  onfile={async ({ pos, file }) => {
                    if (!editor) {
                      return;
                    }

                    if (file.type.startsWith('image/')) {
                      editor.current.chain().focus(pos).setImage().run();
                      const nodeView = getNodeView(editor.current.view, editor.current.state.selection.anchor);

                      const url = URL.createObjectURL(file);
                      nodeView?.handle?.(new CustomEvent('inflight', { detail: { url } }));

                      try {
                        const attrs = await uploadBlobAsImage(file);
                        nodeView?.handle?.(new CustomEvent('success', { detail: { attrs } }));
                      } catch {
                        nodeView?.handle?.(new CustomEvent('error'));
                      } finally {
                        URL.revokeObjectURL(url);
                      }
                    } else {
                      editor?.current.chain().focus(pos).setFile().run();
                      const nodeView = getNodeView(editor.current.view, editor.current.state.selection.anchor);

                      nodeView?.handle?.(new CustomEvent('inflight', { detail: { file } }));

                      try {
                        const attrs = await uploadBlobAsFile(file);
                        nodeView?.handle?.(new CustomEvent('success', { detail: { attrs } }));
                      } catch {
                        nodeView?.handle?.(new CustomEvent('error'));
                      }
                    }
                  }}
                  onkeydown={(view, e) => {
                    const { doc, selection } = view.state;
                    const { anchor } = selection;

                    if (
                      (((!e.altKey && e.key === 'ArrowUp') || (e.key === 'Tab' && e.shiftKey)) && anchor === 2) ||
                      (e.key === 'Backspace' && doc.child(0).childCount === 1 && doc.child(0).child(0).childCount === 0)
                    ) {
                      e.preventDefault();
                      subtitleEl?.focus();
                    }
                  }}
                  bind:editor
                />

                {#if editor && mounted}
                  <Placeholder $site={$query.entity.site} {doc} {editor} />
                  {#if app.preference.current.lineHighlightEnabled}
                    <Highlight {editor} />
                  {/if}
                {/if}
              </div>
            </div>

            {#if showTimeline}
              <div class={css({ position: 'absolute', inset: '0', backgroundColor: 'surface.default', zIndex: '1' })}>
                <Timeline $post={$query.entity.node} {doc} />
              </div>
            {/if}
          </div>

          {#each anchorPositions as anchor (anchor.nodeId)}
            <Anchor name={anchor.name} {editor} element={anchor.element} outline={showAnchorOutline} position={anchor.position} />
          {/each}
        </div>

        {#if app.preference.current.noteExpanded}
          <div
            class={flex({
              flexShrink: '0',
              borderLeftWidth: '1px',
              borderColor: 'border.subtle',
              paddingTop: '16px',
              width: '1/4',
              height: 'full',
              overflowY: 'auto',
              scrollbarGutter: 'stable',
            })}
          >
            <PanelNote {doc} />
          </div>
        {/if}
      </div>
    </div>

    <Panel $post={$query.entity.node} $user={$query.me} {doc} {editor} />
  </div>
{/if}

<Limit {$query} $site={$query.entity.site} {editor} />
