<script lang="ts">
  import { createFragment, createMutation, createQuery } from '@mearie/svelte';
  import { css, cx } from '@typie/styled-system/css';
  import { center, flex, wrap } from '@typie/styled-system/patterns';
  import { createFloatingActions, pointerCapture, portal, tooltip } from '@typie/ui/actions';
  import { Icon, Menu, MenuItem, RingSpinner } from '@typie/ui/components';
  import { getAppContext, getThemeContext } from '@typie/ui/context';
  import { Toast } from '@typie/ui/notification';
  import { clamp, debounce, throttle } from '@typie/ui/utils';
  import dayjs from 'dayjs';
  import mixpanel from 'mixpanel-browser';
  import { onMount, tick } from 'svelte';
  import { fly } from 'svelte/transition';
  import BarChart3Icon from '~icons/lucide/bar-chart-3';
  import ClockRewindIcon from '~icons/lucide/clock-arrow-up';
  import IconClockFading from '~icons/lucide/clock-fading';
  import EllipsisIcon from '~icons/lucide/ellipsis';
  import MinusIcon from '~icons/lucide/minus';
  import PlusIcon from '~icons/lucide/plus';
  import BarChart3OffIcon from '~icons/typie/bar-chart-3-off';
  import { Img } from '$lib/components';
  import { Editor, getEditorContext } from '$lib/editor-ffi/editor.svelte';
  import { cache } from '$lib/graphql';
  import { graphql } from '$mearie';
  import { SubscribeModal } from '../../../@subscription/subscribe-modal.svelte';
  import { getPane, getPaneGroup } from '../../@pane/context.svelte';
  import { getDocumentPanelFocusReturn } from './focus-return.svelte';
  import type { PointerCaptureCancelReason } from '@typie/ui/actions';
  import type { DocumentPanelV2Timeline_document$key } from '$mearie';

  type Props = {
    document$key: DocumentPanelV2Timeline_document$key;
    onPreviewEditorFailed?: (retry: () => void) => void;
    onPreviewEditorRecovered?: () => void;
  };

  const TIMELINE_SLIDER_BOTTOM_OFFSET = 32;

  let { document$key, onPreviewEditorFailed, onPreviewEditorRecovered }: Props = $props();

  const document = createFragment(
    graphql(`
      fragment DocumentPanelV2Timeline_document on Document {
        id

        entity {
          id
          slug
        }
      }
    `),
    () => document$key,
  );

  let queryVars = $state<{ slug: string } | null>(null);

  const query = createQuery(
    graphql(`
      query Editor_DocumentPanelV2Timeline_Query($slug: String!) {
        document(slug: $slug) {
          id
          sweepTombstones

          heads {
            id
            heads
            updatedAt
            excluded
            additions
            deletions

            contributors {
              id
              name
              avatar {
                id
                ...Img_image
              }
            }
          }
        }
      }
    `),
    () => queryVars ?? { slug: '' },
    () => ({ skip: !queryVars }),
  );

  const [revertDocument] = createMutation(
    graphql(`
      mutation Editor_DocumentPanelV2Timeline_RevertDocument($input: RevertDocumentInput!) {
        revertDocument(input: $input) {
          heads
        }
      }
    `),
  );

  const [updateDocumentHeadExclusion] = createMutation(
    graphql(`
      mutation Editor_DocumentPanelV2Timeline_UpdateDocumentHeadExclusion($input: UpdateDocumentHeadExclusionInput!) {
        updateDocumentHeadExclusion(input: $input) {
          id
          excluded
        }
      }
    `),
  );

  const app = getAppContext();
  const ctx = getEditorContext();
  const theme = getThemeContext();
  const pane = getPane();
  const paneGroup = getPaneGroup();
  const focusReturn = getDocumentPanelFocusReturn();

  let timelineEditor = $state<Editor>();
  let creatingTimeline = false;
  let pendingHeadId: string | null = null;
  let pendingTimelinePublication:
    | {
        editor: Editor;
        headId: string;
        afterRevision: number;
      }
    | undefined;
  let previewFailed = $state(false);
  let sliderElement = $state<HTMLButtonElement>();
  let sliderOverlayHeight = $state<number>();
  let destroyed = false;
  const unusableTimelineEditors = new WeakSet<Editor>();
  let restoring = $state(false);

  let selectedHeadId = $state<string | null>(null);
  let shownHeadId = $state<string | null>(null);

  const isLoading = $derived(!query.data);

  const heads = $derived(query.data?.document.heads ?? []);
  const headsAsc = $derived([...heads].toReversed());
  const latestHeadId = $derived(headsAsc.at(-1)?.id ?? null);

  const groupedHeads = $derived.by(() => {
    const dateGroups: Record<string, typeof headsAsc> = {};
    for (const head of headsAsc) {
      const date = dayjs(head.updatedAt).format('YYYY년 M월 D일');
      (dateGroups[date] ??= []).unshift(head);
    }
    const groups = Object.entries(dateGroups).map(([date, list]) => ({ date, heads: list }));
    return groups.toSorted((a, b) => dayjs(b.heads[0].updatedAt).valueOf() - dayjs(a.heads[0].updatedAt).valueOf());
  });

  const { anchor, floating, arrow } = createFloatingActions({ placement: 'top', offset: 8, arrow: true });

  let showTooltip = $state(false);
  let isDraggingSlider = $state(false);

  const sliderIndex = $derived(selectedHeadId ? headsAsc.findIndex((h) => h.id === selectedHeadId) : headsAsc.length - 1);
  const max = $derived(headsAsc.length > 0 ? headsAsc.length - 1 : 0);
  const p = $derived(max > 0 && sliderIndex >= 0 ? `${(sliderIndex / max) * 100}%` : '100%');
  const shownHead = $derived(headsAsc.find((h) => h.id === shownHeadId) ?? null);

  const retryPreview = (): void => {
    if (!selectedHeadId) return;
    updateView.cancel();
    void applyHead(selectedHeadId);
  };

  const failTimeline = (editor?: Editor): void => {
    if (destroyed) return;
    if (editor) {
      if (unusableTimelineEditors.has(editor)) return;
      unusableTimelineEditors.add(editor);
    }
    pendingTimelinePublication = undefined;
    previewFailed = true;
    onPreviewEditorFailed?.(retryPreview);
  };

  const clearTimelineFailure = (): void => {
    previewFailed = false;
    onPreviewEditorRecovered?.();
  };

  const handleTimelineRecovery = (): void => {
    if (previewFailed) sliderElement?.focus({ preventScroll: true });
    clearTimelineFailure();
  };

  const awaitTimelinePublication = (editor: Editor, headId: string, afterRevision: number): void => {
    const pending = { editor, headId, afterRevision };
    pendingTimelinePublication = pending;
    void editor
      .awaitPublishedRevision(pending.afterRevision + 1, { requireFrame: true })
      .then((publication) => {
        if (destroyed || pendingTimelinePublication !== pending || timelineEditor !== editor) return;
        if (publication.type !== 'published') {
          failTimeline(editor);
          return;
        }
        pendingTimelinePublication = undefined;
        shownHeadId = headId;
        handleTimelineRecovery();
      })
      .catch(() => {
        if (pendingTimelinePublication === pending && timelineEditor === editor) failTimeline(editor);
      });
  };

  const exitTimeline = (): void => {
    const exitingEditor = timelineEditor;
    timelineEditor = undefined;
    ctx.editor = ctx.liveEditor;
    creatingTimeline = false;
    pendingHeadId = null;
    pendingTimelinePublication = undefined;
    clearTimelineFailure();
    // The keyed editor subtree still references the previous editor until the next render.
    void tick().then(() => exitingEditor?.destroy());
  };

  onMount(() => {
    queryVars = { slug: document.data.entity.slug };

    return () => {
      destroyed = true;
      scrollToHead.cancel();
      updateView.cancel();
      exitTimeline();
    };
  });

  const scrollToHead = debounce((headId: string) => {
    const element = globalThis.document.querySelector(`[data-panel-timeline-head="${headId}"]`) as HTMLElement;
    element?.scrollIntoView({ behavior: 'smooth', block: 'center' });
  }, 50);

  const applyHead = async (headId: string): Promise<void> => {
    if (destroyed) return;
    const liveEditor = ctx.liveEditor;
    if (!liveEditor || liveEditor.terminal) return;
    const head = headsAsc.find((h) => h.id === headId);
    if (!head) return;
    if (creatingTimeline) {
      pendingHeadId = headId;
      return;
    }

    const currentTimelineEditor = timelineEditor;
    if (
      currentTimelineEditor &&
      currentTimelineEditor.failure === undefined &&
      !unusableTimelineEditors.has(currentTimelineEditor) &&
      ((pendingTimelinePublication?.editor === currentTimelineEditor && pendingTimelinePublication.headId === headId) ||
        (pendingTimelinePublication === undefined && shownHeadId === headId))
    ) {
      return;
    }

    let plain;
    try {
      plain = liveEditor.materializeAt(Uint8Array.fromBase64(head.heads), [...(query.data?.document.sweepTombstones ?? [])]);
    } catch {
      if (liveEditor.failure === undefined) {
        failTimeline(currentTimelineEditor);
      }
      return;
    }

    if (currentTimelineEditor && currentTimelineEditor.failure === undefined && !unusableTimelineEditors.has(currentTimelineEditor)) {
      const afterRevision = currentTimelineEditor.appliedRevision;
      try {
        currentTimelineEditor.setDoc(plain);
      } catch {
        failTimeline(currentTimelineEditor);
        return;
      }
      awaitTimelinePublication(currentTimelineEditor, headId, afterRevision);
      return;
    }

    creatingTimeline = true;
    try {
      let created: Editor;
      try {
        created = await Editor.createFromDoc(plain, liveEditor.viewport, theme.currentThemeVariant);
      } catch {
        failTimeline();
        return;
      }
      created.readOnly = true;
      if (destroyed || ctx.liveEditor !== liveEditor || liveEditor.terminal) {
        created.destroy();
        return;
      }
      const publicationRevision = created.appliedRevision;
      const exitingEditor = timelineEditor;
      timelineEditor = created;
      ctx.editor = created;
      shownHeadId = null;
      await tick();
      exitingEditor?.destroy();
      if (destroyed || timelineEditor !== created) return;

      try {
        const publication = await created.awaitPublishedRevision(publicationRevision, { requireFrame: true });
        if (destroyed || timelineEditor !== created) return;
        if (publication.type !== 'published') {
          failTimeline(created);
          return;
        }
      } catch {
        if (destroyed || timelineEditor !== created) return;
        failTimeline(created);
        return;
      }

      if (pendingHeadId === null || pendingHeadId === headId) {
        if (pendingHeadId === headId) pendingHeadId = null;
        shownHeadId = headId;
        handleTimelineRecovery();
      }
    } finally {
      creatingTimeline = false;
      if (pendingHeadId !== null) {
        const next = pendingHeadId;
        pendingHeadId = null;
        void applyHead(next);
      }
    }
  };
  const updateView = throttle(applyHead, 32);

  $effect(() => {
    const editor = timelineEditor;
    if (editor?.failure !== undefined) failTimeline(editor);
  });

  $effect(() => {
    const scroll = ctx.scroll;
    const editor = ctx.editor;
    const viewport = editor?.scrollViewport;
    const editorAreaElement = ctx.editorAreaEl;
    const overlayHeight = sliderOverlayHeight;
    void editor?.viewport.height;
    if (!scroll || !viewport || !editorAreaElement || !overlayHeight || isLoading || headsAsc.length === 0) return;

    const viewportRect = viewport.getRect();
    const overlayTop = editorAreaElement.getBoundingClientRect().bottom - TIMELINE_SLIDER_BOTTOM_OFFSET - overlayHeight;
    scroll.setBottomInset(clamp(viewportRect.bottom - overlayTop, 0, viewportRect.bottom - viewportRect.top));

    return () => scroll.setBottomInset(0);
  });

  const selectHead = (headId: string, timing: 'throttled' | 'immediate') => {
    const changed = selectedHeadId !== headId;
    if (changed) {
      selectedHeadId = headId;
      scrollToHead.call(headId);
    }

    if (timing === 'throttled') {
      if (changed) updateView.call(headId);
    } else {
      updateView.cancel();
      void applyHead(headId);
    }
  };

  $effect(() => {
    if (selectedHeadId === null && query.data && headsAsc.length > 0) {
      // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
      selectHead(headsAsc.at(-1)!.id, 'immediate');
    }
  });

  const handleSlide = (event: PointerEvent & { currentTarget: HTMLElement }, timing: 'throttled' | 'immediate') => {
    if (!event.currentTarget.parentElement || headsAsc.length === 0) return;
    const { left: parentLeft, width: parentWidth } = event.currentTarget.parentElement.getBoundingClientRect();
    const ratio = clamp((event.clientX - parentLeft) / parentWidth, 0, 1);
    const index = Math.round(ratio * max);
    if (Object.hasOwn(headsAsc, index)) selectHead(headsAsc[index].id, timing);
  };

  const handleSlideStart = (event: PointerEvent): true | null => {
    if (showTooltip || isDraggingSlider || !event.isPrimary || event.button !== 0) return null;
    handleSlide(event as PointerEvent & { currentTarget: HTMLElement }, 'throttled');
    showTooltip = true;
    return true;
  };

  const handleSlideMove = (_: true, event: PointerEvent) => {
    event.preventDefault();
    isDraggingSlider = true;
    handleSlide(event as PointerEvent & { currentTarget: HTMLElement }, 'throttled');
  };

  const handleSlideEnd = (_: true, event: PointerEvent) => {
    handleSlide(event as PointerEvent & { currentTarget: HTMLElement }, 'immediate');
    showTooltip = false;
    isDraggingSlider = false;
  };

  const handleSlideCancel = (_: true, reason: PointerCaptureCancelReason) => {
    showTooltip = false;
    isDraggingSlider = false;
    if (reason !== 'destroy' && selectedHeadId) {
      selectHead(selectedHeadId, 'immediate');
    } else {
      updateView.cancel();
    }
  };

  const restore = async (head: (typeof headsAsc)[number] | null) => {
    if (!head || restoring) return;

    if (!SubscribeModal.gate('document_revert')) {
      return;
    }

    restoring = true;
    try {
      await revertDocument({ input: { documentId: document.data.id, headId: head.id } });
    } catch {
      Toast.error('복원에 실패했어요. 잠시 후 다시 시도해 주세요.');
      return;
    } finally {
      restoring = false;
    }

    exitTimeline();
    paneGroup.state.current.panelExpandedByPaneId[pane.id] = false;
    focusReturn.restore();
    Toast.success(`${dayjs(head.updatedAt).formatAsSmart()} 시점으로 복원되었습니다`);
    mixpanel.track('document_timeline_restore');
  };

  const toggleExclusion = async (headId: string, excluded: boolean) => {
    if (!SubscribeModal.gate('document_head_exclusion')) {
      return;
    }

    try {
      await updateDocumentHeadExclusion({ input: { headId, excluded: !excluded } });
    } catch {
      Toast.error('통계 제외 설정에 실패했어요. 잠시 후 다시 시도해 주세요.');
      return;
    }

    cache.invalidate(
      { __typename: 'User', id: app.userId, $field: 'characterCountChanges' },
      { __typename: 'User', id: app.userId, $field: 'goalHistory' },
      { __typename: 'User', id: app.userId, $field: 'todayCharacterCountChange' },
      { __typename: 'Document', id: document.data.id, $field: 'characterCountChange' },
    );
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
      alignItems: 'center',
      height: '41px',
      paddingX: '20px',
      fontSize: '13px',
      fontWeight: 'semibold',
      color: 'text.muted',
      borderBottomWidth: '1px',
      borderColor: 'border.hairline',
    })}
  >
    타임라인
  </div>

  <div class={flex({ flexDirection: 'column', flex: '1', overflow: 'auto' })}>
    {#if isLoading}
      <div class={center({ padding: '32px' })}>
        <RingSpinner style={css.raw({ size: '24px', color: 'text.muted' })} />
      </div>
    {:else if headsAsc.length === 0}
      <div class={center({ padding: '32px', flexDirection: 'column', gap: '8px', color: 'text.muted', fontSize: '13px' })}>
        <Icon style={css.raw({ color: 'text.muted' })} icon={IconClockFading} size={24} />
        아직 버전 기록이 없습니다
      </div>
    {:else}
      <div class={flex({ flexDirection: 'column' })}>
        {#each groupedHeads as group (group.date)}
          <div class={flex({ flexDirection: 'column' })}>
            <div
              class={css({
                position: 'sticky',
                top: '0',
                padding: '8px',
                paddingX: '20px',
                backgroundColor: 'surface.canvas',
                borderBottomWidth: '1px',
                borderColor: 'border.hairline',
                fontSize: '12px',
                fontWeight: 'semibold',
                color: 'text.muted',
                zIndex: '1',
              })}
            >
              {group.date}
            </div>

            {#each group.heads as head (head.id)}
              {@const isSelected = selectedHeadId === head.id}
              {@const time = dayjs(head.updatedAt)}
              {@const excluded = head.excluded ?? null}
              <div
                class={flex({
                  alignItems: 'center',
                  backgroundColor: isSelected ? 'surface.active' : 'transparent',
                  borderLeftWidth: '3px',
                  borderLeftColor: isSelected ? 'accent.default' : 'transparent',
                  transition: 'all',
                  transitionDuration: '150ms',
                  _hover: { backgroundColor: 'surface.hover' },
                })}
                aria-current={isSelected ? 'true' : undefined}
              >
                <button
                  class={css({
                    display: 'flex',
                    alignItems: 'center',
                    gap: '12px',
                    flex: '1',
                    minWidth: '0',
                    paddingY: '10px',
                    paddingX: '14px',
                    cursor: 'pointer',
                  })}
                  data-panel-timeline-head={head.id}
                  onclick={() => selectHead(head.id, 'immediate')}
                  type="button"
                >
                  <Icon
                    style={css.raw({ flexShrink: '0', color: isSelected ? 'accent.default' : 'text.muted' })}
                    icon={ClockRewindIcon}
                    size={14}
                  />

                  <div class={flex({ flexDirection: 'column', align: 'start', gap: '2px', flex: '1', minWidth: '0' })}>
                    <div class={flex({ alignItems: 'center', gap: '8px', minWidth: '0', overflow: 'hidden', whiteSpace: 'nowrap' })}>
                      <div
                        class={css({
                          flexShrink: '0',
                          fontSize: '13px',
                          fontWeight: isSelected ? 'medium' : 'normal',
                          color: 'text.default',
                        })}
                      >
                        {time.format('H시 mm분 ss초')}
                      </div>
                      {#if excluded === true}
                        <div
                          class={center({ flexShrink: '0' })}
                          aria-label="통계 제외됨"
                          use:tooltip={{ message: '통계 제외됨', placement: 'top' }}
                        >
                          <Icon style={css.raw({ color: 'text.muted' })} icon={BarChart3OffIcon} size={12} />
                        </div>
                      {/if}
                    </div>
                    <div class={flex({ alignItems: 'center', gap: '6px', minWidth: '0', overflow: 'hidden', whiteSpace: 'nowrap' })}>
                      <div class={css({ flexShrink: '0', fontSize: '11px', color: 'text.muted' })}>
                        {time.fromNow()}
                      </div>
                      {#if head.additions}
                        <div class={center({ flexShrink: '0' })} in:fly={{ y: 10, duration: 150 }}>
                          <Icon style={css.raw({ size: '10px', color: 'success.default' })} icon={PlusIcon} />
                          <span class={css({ fontSize: '11px', fontWeight: 'medium', color: 'success.default' })}>
                            {head.additions.toLocaleString()}
                          </span>
                        </div>
                      {/if}
                      {#if head.deletions}
                        <div class={center({ flexShrink: '0' })} in:fly={{ y: 10, duration: 150 }}>
                          <Icon style={css.raw({ size: '10px', color: 'danger.default' })} icon={MinusIcon} />
                          <span class={css({ fontSize: '11px', fontWeight: 'medium', color: 'danger.default' })}>
                            {head.deletions.toLocaleString()}
                          </span>
                        </div>
                      {/if}
                      {#if head.contributors.length > 0}
                        <div class={flex({ alignItems: 'center', minWidth: '0', overflow: 'hidden' })}>
                          {#each head.contributors.slice(0, 3) as contributor (contributor.id)}
                            <div
                              class={css({
                                flexShrink: '0',
                                width: '14px',
                                height: '14px',
                                aspectRatio: '1/1',
                                overflow: 'hidden',
                                borderRadius: 'full',
                                marginLeft: '-3px',
                                borderWidth: '1px',
                                borderColor: 'surface.default',
                                _first: { marginLeft: '0' },
                              })}
                            >
                              <Img
                                style={css.raw({ size: 'full', objectFit: 'cover' })}
                                alt={contributor.name}
                                image$key={contributor.avatar}
                                size={16}
                              />
                            </div>
                          {/each}
                        </div>
                      {/if}
                    </div>
                  </div>
                </button>

                <Menu style={css.raw({ flexShrink: '0', marginRight: '14px', marginLeft: '4px' })} placement="bottom-end">
                  {#snippet button({ open })}
                    <div
                      class={center({
                        borderRadius: '4px',
                        size: '20px',
                        color: 'text.muted',
                        transition: 'common',
                        _hover: { backgroundColor: 'surface.hover' },
                        _pressed: { backgroundColor: 'surface.active', color: 'text.default' },
                      })}
                      aria-pressed={open}
                    >
                      <Icon icon={EllipsisIcon} size={14} />
                    </div>
                  {/snippet}

                  <MenuItem icon={IconClockFading} onclick={() => restore(head)}>이 버전으로 복원</MenuItem>

                  {#if excluded !== null}
                    <MenuItem icon={BarChart3Icon} onclick={() => toggleExclusion(head.id, excluded)}>
                      {excluded ? '통계에 포함' : '통계에서 제외'}
                    </MenuItem>
                  {/if}
                </Menu>
              </div>
            {/each}
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>

{#if ctx.editorAreaEl && !isLoading && headsAsc.length > 0}
  <div
    style:bottom={`${TIMELINE_SLIDER_BOTTOM_OFFSET}px`}
    class={center({ position: 'absolute', left: '0', right: '0', pointerEvents: 'none' })}
    bind:clientHeight={sliderOverlayHeight}
    use:portal={ctx.editorAreaEl}
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
        backgroundColor: 'surface.default',
        border: '1px solid',
        borderColor: 'border.default',
        boxShadow: 'xl',
        zIndex: 'overEditor',
        pointerEvents: 'auto',
      })}
    >
      <Icon style={css.raw({ color: 'text.muted' })} icon={IconClockFading} size={18} />

      <div class={flex({ position: 'relative', flexGrow: '1', align: 'center', minWidth: '100px', maxWidth: '420px', height: '36px' })}>
        <button
          bind:this={sliderElement}
          class={cx('group', css({ position: 'relative', width: 'full', height: '16px', overflow: 'hidden', cursor: 'pointer' }))}
          aria-label="Timeline slider"
          type="button"
          use:pointerCapture={{
            start: handleSlideStart,
            move: handleSlideMove,
            end: handleSlideEnd,
            cancel: handleSlideCancel,
          }}
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
              backgroundColor: 'surface.inset',
              transition: 'all',
              transitionDuration: isDraggingSlider ? '0ms' : '150ms',
              _groupHover: { backgroundColor: 'surface.hover' },
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
              backgroundColor: 'accent.default',
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
              boxShadow: 'md',
              _hover: { scale: '[1.2]', boxShadow: 'lg' },
              _active: { scale: '[1.1]' },
            })}
            use:anchor
            use:pointerCapture={{
              start: handleSlideStart,
              move: handleSlideMove,
              end: handleSlideEnd,
              cancel: handleSlideCancel,
            }}
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
        {dayjs(shownHead?.updatedAt).formatAsSmart()}
      </div>

      <div
        class={css({
          paddingX: '12px',
          paddingY: '8px',
          backgroundColor: 'surface.inverse',
          borderRadius: '8px',
          zIndex: 'overEditor',
          fontSize: '12px',
          color: 'text.on.inverse',
          whiteSpace: 'nowrap',
          pointerEvents: 'none',
          opacity: showTooltip ? '100' : '0',
          transition: 'opacity',
          transitionDuration: '150ms',
          boxShadow: 'lg',
        })}
        role="tooltip"
        use:floating
      >
        {dayjs(headsAsc[sliderIndex]?.updatedAt).formatAsSmart()}
        <div class={css({ size: '6px', backgroundColor: 'surface.inverse', zIndex: 'overEditor' })} use:arrow></div>
      </div>

      {#if shownHeadId === null || shownHeadId === latestHeadId}
        <div
          class={center({
            flexShrink: '0',
            width: '75px',
            gap: '6px',
            paddingY: '8px',
            backgroundColor: 'accent.subtle',
            color: 'text.default',
            borderRadius: '8px',
            fontSize: '13px',
            fontWeight: 'semibold',
            cursor: 'default',
            userSelect: 'none',
          })}
          use:tooltip={{ message: '현재 최신 버전을 보고 있습니다', placement: 'top' }}
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
            backgroundColor: 'accent.default',
            color: 'surface.default',
            borderRadius: '8px',
            fontSize: '13px',
            fontWeight: 'medium',
            cursor: 'pointer',
            transition: 'all',
            transitionDuration: '150ms',
            _hover: { backgroundColor: '[color-mix(in oklch, token(colors.accent.default) 88%, black)]', transform: 'translateY(-1px)' },
            _active: { backgroundColor: '[color-mix(in oklch, token(colors.accent.default) 80%, black)]', transform: 'translateY(0)' },
            _disabled: { cursor: 'default', backgroundColor: 'accent.default', transform: 'none' },
          })}
          aria-busy={restoring}
          disabled={restoring}
          onclick={() => restore(shownHead)}
          type="button"
          use:tooltip={{ message: '이 시점으로 문서를 복원하고 타임라인에 새로 추가합니다', placement: 'top' }}
        >
          {#if restoring}
            <RingSpinner style={css.raw({ size: '14px', color: 'surface.default' })} />
          {:else}
            <Icon style={css.raw({ size: '14px' })} icon={IconClockFading} />
          {/if}
          복원
        </button>
      {/if}
    </div>
  </div>
{/if}
