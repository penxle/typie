<script lang="ts">
  import { getText } from '@tiptap/core';
  import { css, cx } from '@typie/styled-system/css';
  import { center, flex, wrap } from '@typie/styled-system/patterns';
  import { createFloatingActions, portal, tooltip } from '@typie/ui/actions';
  import { Icon, RingSpinner } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import { Toast } from '@typie/ui/notification';
  import { getEditorContext } from '@typie/ui/tiptap';
  import { clamp, debounce, throttle } from '@typie/ui/utils';
  import dayjs from 'dayjs';
  import mixpanel from 'mixpanel-browser';
  import { base64 } from 'rfc4648';
  import { onMount, tick, untrack } from 'svelte';
  import { on } from 'svelte/events';
  import { fly } from 'svelte/transition';
  import { yXmlFragmentToProseMirrorRootNode } from 'y-prosemirror';
  import * as Y from 'yjs';
  import { PostLayoutMode } from '@/enums';
  import { schema } from '@/pm';
  import { textSerializers } from '@/pm/serializer';
  import ClockRewindIcon from '~icons/lucide/clock-arrow-up';
  import IconClockFading from '~icons/lucide/clock-fading';
  import MinusIcon from '~icons/lucide/minus';
  import PlusIcon from '~icons/lucide/plus';
  import { fragment, graphql } from '$graphql';
  import { getViewContext } from '../@split-view/context.svelte';
  import type { Editor } from '@tiptap/core';
  import type { PageLayout, Ref } from '@typie/ui/utils';
  import type { Action } from 'svelte/action';
  import type { PointerEventHandler } from 'svelte/elements';
  import type { Editor_PanelTimeline_post } from '$graphql';

  type Props = {
    $post: Editor_PanelTimeline_post;
    editor?: Ref<Editor>;
    viewEditor?: Ref<Editor>;
    doc: Y.Doc;
    viewDoc?: Y.Doc;
  };

  let { $post: _post, editor, viewEditor, doc, viewDoc = $bindable() }: Props = $props();

  const post = fragment(
    _post,
    graphql(`
      fragment Editor_PanelTimeline_post on Post {
        id

        entity {
          id
          slug
        }
      }
    `),
  );

  const query = graphql(`
    query Editor_PanelTimeline_Query($slug: String!) @client {
      post(slug: $slug) {
        id
        update

        snapshots {
          id
          snapshot
          createdAt
        }
      }
    }
  `);

  const app = getAppContext();
  const view = getViewContext();
  const editorContext = getEditorContext();

  const editorContainer = $derived((editor?.current.view.dom.closest('.editor-scroll-container') as HTMLElement)?.parentElement);

  let selectedSnapshotId = $state<string | null>(null);
  let isLoading = $state(true);
  let baseDoc: Y.Doc | null = null;
  let snapshotCharCounts = $state<Record<string, number>>({});
  let internalViewDoc = $state<Y.Doc>();

  const snapshots = $derived($query?.post.snapshots ?? []);

  const groupedSnapshots = $derived.by(() => {
    const groups: { date: string; snapshots: typeof snapshots }[] = [];
    const dateGroups: Record<string, typeof snapshots> = {};

    snapshots.forEach((snapshot) => {
      const date = dayjs(snapshot.createdAt).format('YYYY년 M월 D일');
      if (!dateGroups[date]) {
        dateGroups[date] = [];
      }
      dateGroups[date].unshift(snapshot);
    });

    Object.entries(dateGroups).forEach(([date, snapshotList]) => {
      groups.push({ date, snapshots: snapshotList });
    });

    return groups.sort((a, b) => dayjs(b.snapshots[0].createdAt).valueOf() - dayjs(a.snapshots[0].createdAt).valueOf());
  });

  const { anchor, floating, arrow } = createFloatingActions({
    placement: 'top',
    offset: 8,
    arrow: true,
  });

  let showTooltip = $state(false);
  let isDraggingSlider = $state(false);

  const sliderIndex = $derived(selectedSnapshotId ? snapshots.findIndex((s) => s.id === selectedSnapshotId) : snapshots.length - 1);
  const max = $derived($query ? $query.post.snapshots.length - 1 : 0);
  const p = $derived(max > 0 && sliderIndex >= 0 ? `${(sliderIndex / max) * 100}%` : '0%');

  // NOTE: 서버와 동일한 글자수 세기 로직
  const getCharacterCount = (doc: Y.Doc): number => {
    const xmlFragment = doc.getXmlFragment('body');
    const node = yXmlFragmentToProseMirrorRootNode(xmlFragment, schema);

    const text = getText(node, {
      blockSeparator: '\n',
      textSerializers,
    }).trim();

    return [...text.replaceAll(/\s+/g, ' ').trim()].length;
  };

  const initialize = async () => {
    isLoading = true;
    try {
      const result = await query.load({ slug: $post.entity.slug });

      baseDoc = new Y.Doc({ gc: false });
      Y.applyUpdateV2(baseDoc, base64.parse(result.post.update));

      const counts: Record<string, number> = {};
      const snapshots = [...result.post.snapshots].toReversed();

      selectedSnapshotId = snapshots[0].id;
      let currentIndex = 0;

      const processNextSnapshot = (deadline: IdleDeadline) => {
        if (!baseDoc) {
          return;
        }

        while (currentIndex < snapshots.length && deadline.timeRemaining() > 0) {
          const snapshot = snapshots[currentIndex];
          const snapshotData = Y.decodeSnapshotV2(base64.parse(snapshot.snapshot));
          const snapshotDoc = Y.createDocFromSnapshot(baseDoc, snapshotData);
          counts[snapshot.id] = getCharacterCount(snapshotDoc);
          currentIndex++;

          snapshotCharCounts = { ...counts };
        }

        if (currentIndex < snapshots.length) {
          requestIdleCallback(processNextSnapshot);
        }
      };

      if ('requestIdleCallback' in window) {
        requestIdleCallback(processNextSnapshot);
      } else {
        // NOTE: Safari 등 requestIdleCallback이 없는 경우 fallback
        const processChunk = () => {
          if (!baseDoc) {
            return;
          }

          const chunkSize = 5;
          for (let i = 0; i < chunkSize && currentIndex < snapshots.length; i++) {
            const snapshot = snapshots[currentIndex];
            const snapshotData = Y.decodeSnapshotV2(base64.parse(snapshot.snapshot));
            const snapshotDoc = Y.createDocFromSnapshot(baseDoc, snapshotData);
            counts[snapshot.id] = getCharacterCount(snapshotDoc);
            currentIndex++;
          }

          snapshotCharCounts = { ...counts };

          if (currentIndex < snapshots.length) {
            setTimeout(processChunk);
          }
        };

        setTimeout(processChunk);
      }
    } finally {
      tick().then(() => {
        isLoading = false;
      });
    }
  };

  onMount(() => {
    if (editorContext) {
      editorContext.timeline = true;
    }
    initialize();

    return () => {
      if (editorContext) {
        editorContext.timeline = false;
      }
      viewDoc?.destroy();
      viewDoc = undefined;
      internalViewDoc?.destroy();
      internalViewDoc = undefined;
      baseDoc?.destroy();
      baseDoc = null;
      updateViewDoc.cancel();
    };
  });

  const scrollToSnapshot = debounce((snapshotId: string) => {
    const element = document.querySelector(`[data-panel-timeline-snapshot="${snapshotId}"]`) as HTMLElement;
    element?.scrollIntoView({ behavior: 'smooth', block: 'center' });
  }, 50);

  $effect(() => {
    if (selectedSnapshotId && $query) {
      scrollToSnapshot(selectedSnapshotId);
      updateViewDoc.call(selectedSnapshotId);
    }
  });

  const updateViewDoc = throttle((snapshotId: string) => {
    if (!baseDoc || !$query) return;

    const snapshot = $query.post.snapshots.find((s) => s.id === snapshotId);
    if (!snapshot) return;

    if (!internalViewDoc) {
      internalViewDoc = new Y.Doc({ gc: false });
    }

    const snapshotData = Y.decodeSnapshotV2(base64.parse(snapshot.snapshot));
    const snapshotDoc = Y.createDocFromSnapshot(baseDoc, snapshotData);

    const currentStateVector = Y.encodeStateVector(internalViewDoc);
    const snapshotStateVector = Y.encodeStateVector(snapshotDoc);

    const missingUpdate = Y.encodeStateAsUpdateV2(internalViewDoc, snapshotStateVector);

    const undoManager = new Y.UndoManager(snapshotDoc, { trackedOrigins: new Set(['snapshot']) });
    Y.applyUpdateV2(snapshotDoc, missingUpdate, 'snapshot');
    undoManager.undo();

    const revertUpdate = Y.encodeStateAsUpdateV2(snapshotDoc, currentStateVector);
    Y.applyUpdateV2(internalViewDoc, revertUpdate, 'snapshot');

    viewDoc = internalViewDoc;

    untrack(() => {
      tick().then(() => {
        if (viewEditor?.current) {
          const attrs = snapshotDoc.getMap('attrs');
          const layoutMode = (attrs.get('layoutMode') as PostLayoutMode) ?? PostLayoutMode.SCROLL;
          const pageLayout = (attrs.get('pageLayout') as PageLayout) ?? null;
          if (layoutMode === PostLayoutMode.PAGE && pageLayout) {
            viewEditor.current.commands.setPageLayout(pageLayout);
          } else {
            viewEditor.current.commands.clearPageLayout();
          }
        }
      });
    });
  }, 32);

  const handleSlide: PointerEventHandler<HTMLElement> = (e) => {
    if (!e.currentTarget.parentElement || !$query) {
      return;
    }

    const { left: parentLeft, width: parentWidth } = e.currentTarget.parentElement.getBoundingClientRect();
    const { clientX: pointerLeft } = e;
    const ratio = clamp((pointerLeft - parentLeft) / parentWidth, 0, 1);
    const index = Math.round(ratio * max);

    if (snapshots[index]) {
      selectedSnapshotId = snapshots[index].id;
    }
  };

  const restore = () => {
    if (!$query || !baseDoc) {
      return;
    }

    const snapshot = Y.decodeSnapshotV2(base64.parse($query.post.snapshots[sliderIndex].snapshot));
    const snapshotDoc = Y.createDocFromSnapshot(baseDoc, snapshot);

    const currentStateVector = Y.encodeStateVector(doc);
    const snapshotStateVector = Y.encodeStateVector(snapshotDoc);

    const missingUpdate = Y.encodeStateAsUpdateV2(doc, snapshotStateVector);

    const undoManager = new Y.UndoManager(snapshotDoc, { trackedOrigins: new Set(['snapshot']) });
    Y.applyUpdateV2(snapshotDoc, missingUpdate, 'snapshot');
    undoManager.undo();

    const revertUpdate = Y.encodeStateAsUpdateV2(snapshotDoc, currentStateVector);
    Y.applyUpdateV2(doc, revertUpdate, 'snapshot');

    app.preference.current.panelExpandedByViewId[view.id] = false;
    Toast.success(`${dayjs($query.post.snapshots[sliderIndex]?.createdAt).formatAsSmart()} 시점으로 복원되었습니다`);
    mixpanel.track('timeline_restore');
  };

  const slider: Action<HTMLElement> = (element) => {
    $effect(() => {
      const dragstart = on(element, 'dragstart', (e) => {
        e.preventDefault();
      });
      const pointerdown = on(element, 'pointerdown', (e) => {
        handleSlide(e as PointerEvent & { currentTarget: HTMLElement });
        showTooltip = true;
        (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
      });
      const pointermove = on(element, 'pointermove', (e) => {
        e.preventDefault();
        if ((e.currentTarget as HTMLElement).hasPointerCapture(e.pointerId)) {
          isDraggingSlider = true;
          handleSlide(e as PointerEvent & { currentTarget: HTMLElement });
        }
      });
      const pointerup = on(element, 'pointerup', () => {
        showTooltip = false;
        isDraggingSlider = false;
      });

      return () => {
        dragstart();
        pointerdown();
        pointermove();
        pointerup();
      };
    });
  };
</script>

<div
  class={flex({
    flexDirection: 'column',
    minWidth: 'var(--min-width)',
    width: 'var(--width)',
    maxWidth: 'var(--max-width)',
    height: 'full',
  })}
>
  <div
    class={flex({
      flexShrink: '0',
      height: '40px',
      alignItems: 'center',
      paddingX: '20px',
      fontSize: '13px',
      fontWeight: 'semibold',
      color: 'text.subtle',
      borderBottomWidth: '1px',
      borderColor: 'surface.muted',
    })}
  >
    타임라인
  </div>

  <div class={flex({ flexDirection: 'column', flex: '1', overflow: 'auto' })}>
    {#if isLoading}
      <div class={center({ padding: '32px' })}>
        <RingSpinner style={css.raw({ size: '24px', color: 'text.subtle' })} />
      </div>
    {:else}
      <div class={flex({ flexDirection: 'column' })}>
        {#each groupedSnapshots as group (group.date)}
          <div class={flex({ flexDirection: 'column' })}>
            <div
              class={css({
                position: 'sticky',
                top: '0',
                padding: '8px',
                paddingX: '20px',
                backgroundColor: 'surface.subtle',
                borderBottomWidth: '1px',
                borderColor: 'surface.muted',
                fontSize: '12px',
                fontWeight: 'semibold',
                color: 'text.subtle',
                zIndex: '1',
              })}
            >
              {group.date}
            </div>

            {#each group.snapshots as snapshot (snapshot.id)}
              {@const isSelected = selectedSnapshotId === snapshot.id}
              {@const time = dayjs(snapshot.createdAt)}
              {@const currentCount = snapshotCharCounts[snapshot.id] ?? null}
              {@const snapshotIndex = snapshots.findIndex((s) => s.id === snapshot.id)}
              {@const prevSnapshot = snapshotIndex > 0 ? snapshots[snapshotIndex - 1] : null}
              {@const prevCount =
                prevSnapshot && snapshotCharCounts[prevSnapshot.id] !== undefined ? snapshotCharCounts[prevSnapshot.id] : null}
              {@const charDiff =
                snapshotIndex === 0 ? currentCount : prevCount === null || currentCount === null ? null : currentCount - prevCount}
              <button
                class={css({
                  display: 'flex',
                  alignItems: 'center',
                  gap: '12px',
                  paddingY: '10px',
                  paddingX: '14px',
                  backgroundColor: isSelected ? 'surface.muted' : 'transparent',
                  borderLeftWidth: '3px',
                  borderLeftColor: isSelected ? 'accent.brand.default' : 'transparent',
                  cursor: 'pointer',
                  transition: 'all',
                  transitionDuration: '150ms',
                  _hover: {
                    backgroundColor: isSelected ? 'surface.muted' : 'surface.subtle',
                  },
                  _focusVisible: {
                    backgroundColor: isSelected ? 'surface.muted' : 'surface.subtle',
                    outline: 'none',
                  },
                })}
                data-panel-timeline-snapshot={snapshot.id}
                onclick={() => {
                  selectedSnapshotId = snapshot.id;
                }}
                onkeydown={(e) => {
                  if (e.key === 'Enter' || e.key === ' ') {
                    e.preventDefault();
                    selectedSnapshotId = snapshot.id;
                  } else if (e.key === 'ArrowUp') {
                    e.preventDefault();
                    const currentIndex = snapshots.findIndex((s) => s.id === snapshot.id);
                    const nextSnapshot = snapshots[currentIndex + 1];
                    if (nextSnapshot) {
                      selectedSnapshotId = nextSnapshot.id;
                      tick().then(() => {
                        const nextElement = document.querySelector(`[data-panel-timeline-snapshot="${nextSnapshot.id}"]`) as HTMLElement;
                        nextElement?.focus();
                      });
                    }
                  } else if (e.key === 'ArrowDown') {
                    e.preventDefault();
                    const currentIndex = snapshots.findIndex((s) => s.id === snapshot.id);
                    const prevSnapshot = snapshots[currentIndex - 1];
                    if (prevSnapshot) {
                      selectedSnapshotId = prevSnapshot.id;
                      tick().then(() => {
                        const prevElement = document.querySelector(`[data-panel-timeline-snapshot="${prevSnapshot.id}"]`) as HTMLElement;
                        prevElement?.focus();
                      });
                    }
                  }
                }}
                type="button"
              >
                <Icon
                  style={css.raw({
                    flexShrink: '0',
                    color: isSelected ? 'accent.brand.default' : 'text.subtle',
                  })}
                  icon={ClockRewindIcon}
                  size={14}
                />

                <div class={flex({ flexDirection: 'column', align: 'start', gap: '2px', flex: '1' })}>
                  <div class={flex({ alignItems: 'center', gap: '8px' })}>
                    <div class={css({ fontSize: '13px', fontWeight: isSelected ? 'medium' : 'normal', color: 'text.default' })}>
                      {time.format('H시 mm분 ss초')}
                    </div>
                    {#if charDiff !== null && charDiff !== 0}
                      <div class={center()} in:fly={{ y: 10, duration: 150 }}>
                        <Icon
                          style={css.raw({
                            size: '10px',
                            color: charDiff > 0 ? 'text.success' : 'text.danger',
                          })}
                          icon={charDiff > 0 ? PlusIcon : MinusIcon}
                        />
                        <span
                          class={css({
                            fontSize: '11px',
                            fontWeight: 'medium',
                            color: charDiff > 0 ? 'text.success' : 'text.danger',
                          })}
                        >
                          {Math.abs(charDiff).toLocaleString()}
                        </span>
                      </div>
                    {/if}
                  </div>
                  <div class={css({ fontSize: '11px', color: 'text.subtle' })}>
                    {time.fromNow()}
                  </div>
                </div>
              </button>
            {/each}
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>

{#if editorContainer && $query && !isLoading}
  <div
    class={center({ position: 'absolute', left: '0', right: '0', bottom: '32px' })}
    use:portal={editorContainer}
    in:fly={{ y: 32, duration: 300 }}
  >
    <div
      class={wrap({
        width: 'full',
        marginX: '16px',
        minWidth: 'fit',
        maxWidth: '650px',
        align: 'center',
        columnGap: '16px',
        rowGap: '6px',
        borderRadius: '12px',
        padding: '12px',
        paddingRight: '16px',
        backgroundColor: 'surface.subtle',
        border: '1px solid',
        borderColor: 'border.default',
        boxShadow: '[0 8px 32px rgba(0,0,0,0.08)]',
        zIndex: 'overEditor',
      })}
    >
      <Icon style={css.raw({ color: 'gray.500' })} icon={IconClockFading} size={18} />

      <div
        class={flex({
          position: 'relative',
          flexGrow: '1',
          align: 'center',
          minWidth: '100px',
          maxWidth: '420px',
          height: '36px',
        })}
      >
        <button
          class={cx(
            'group',
            css({
              position: 'relative',
              width: 'full',
              height: '16px',
              overflow: 'hidden',
              cursor: 'pointer',
            }),
          )}
          aria-label="Timeline slider"
          type="button"
          use:slider
        >
          <div
            class={css({
              position: 'absolute',
              top: '1/2',
              left: '0',
              translate: 'auto',
              translateY: '-1/2',
              width: 'full',
              height: '4px',
              borderRadius: 'full',
              backgroundColor: { base: 'gray.500', _dark: 'dark.gray.500' },
              transition: 'all',
              transitionDuration: isDraggingSlider ? '0ms' : '150ms',
              _groupHover: {
                backgroundColor: { base: 'gray.400', _dark: 'dark.gray.600' },
              },
            })}
          ></div>
          <div
            style:width={p}
            class={css({
              position: 'absolute',
              top: '1/2',
              left: '0',
              translate: 'auto',
              translateY: '-1/2',
              height: '4px',
              borderRadius: 'full',
              backgroundColor: 'accent.brand.default',
              transition: 'all',
              transitionDuration: isDraggingSlider ? '0ms' : '150ms',
              transitionTimingFunction: 'cubic-bezier(0.4, 0, 0.2, 1)',
            })}
          ></div>
        </button>
        <div class={css({ position: 'absolute', width: 'full', height: '36px', pointerEvents: 'none' })}>
          <div
            style:left={p}
            class={css({
              position: 'absolute',
              top: '1/2',
              borderRadius: 'full',
              size: '16px',
              backgroundColor: 'surface.default',
              border: '2px solid',
              borderColor: 'border.default',
              translate: 'auto',
              translateX: '-1/2',
              translateY: '-1/2',
              pointerEvents: 'auto',
              touchAction: 'none',
              cursor: 'ew-resize',
              transition: 'all',
              transitionDuration: isDraggingSlider ? '0ms' : '150ms',
              boxShadow: '[0 2px 8px rgba(0,0,0,0.1)]',
              _hover: {
                scale: '[1.2]',
                boxShadow: '[0 4px 12px rgba(0,0,0,0.15)]',
              },
              _active: {
                scale: '[1.1]',
              },
            })}
            use:anchor
            use:slider
          ></div>
        </div>
      </div>

      <div
        class={css({
          fontSize: '13px',
          fontFeatureSettings: '"tnum" 1',
          color: 'text.default',
          whiteSpace: 'nowrap',
          minWidth: '150px',
          textAlign: 'center',
        })}
      >
        {dayjs($query.post.snapshots[sliderIndex]?.createdAt).formatAsSmart()}
      </div>

      <div
        class={css({
          paddingX: '12px',
          paddingY: '8px',
          backgroundColor: 'surface.dark',
          borderRadius: '8px',
          zIndex: 'overEditor',
          fontSize: '12px',
          color: 'text.bright',
          whiteSpace: 'nowrap',
          pointerEvents: 'none',
          opacity: showTooltip ? '100' : '0',
          transition: 'opacity',
          transitionDuration: '150ms',
          boxShadow: '[0 4px 12px rgba(0,0,0,0.15)]',
        })}
        role="tooltip"
        use:floating
      >
        {dayjs($query.post.snapshots[sliderIndex]?.createdAt).formatAsSmart()}
        <div
          class={css({
            size: '6px',
            backgroundColor: 'surface.dark',
            zIndex: 'overEditor',
          })}
          use:arrow
        ></div>
      </div>

      {#if selectedSnapshotId === $query?.post.snapshots.at(-1)?.id}
        <div
          class={center({
            flexShrink: '0',
            width: '75px',
            gap: '6px',
            paddingY: '8px',
            backgroundColor: 'accent.brand.subtle',
            color: 'accent.brand.default',
            borderRadius: '8px',
            fontSize: '13px',
            fontWeight: 'semibold',
            cursor: 'default',
            userSelect: 'none',
          })}
          use:tooltip={{
            message: '현재 최신 버전을 보고 있습니다',
            placement: 'top',
          }}
        >
          최신 버전
        </div>
      {:else}
        <button
          class={center({
            flexShrink: '0',
            gap: '6px',
            width: '75px',
            paddingY: '8px',
            backgroundColor: 'accent.brand.default',
            color: 'text.bright',
            borderRadius: '8px',
            fontSize: '13px',
            fontWeight: 'medium',
            cursor: 'pointer',
            transition: 'all',
            transitionDuration: '150ms',
            _hover: {
              backgroundColor: 'accent.brand.hover',
              transform: 'translateY(-1px)',
            },
            _active: {
              backgroundColor: 'accent.brand.active',
              transform: 'translateY(0)',
            },
          })}
          onclick={restore}
          type="button"
          use:tooltip={{
            message: '이 시점으로 포스트를 복원하고 타임라인에 새로 추가합니다',
            placement: 'top',
          }}
        >
          <Icon style={css.raw({ size: '14px' })} icon={IconClockFading} />
          복원
        </button>
      {/if}
    </div>
  </div>
{/if}
