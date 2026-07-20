<script lang="ts">
  import { createFragment, createMutation, createQuery } from '@mearie/svelte';
  import { css, cx } from '@typie/styled-system/css';
  import { center, flex, wrap } from '@typie/styled-system/patterns';
  import { createFloatingActions, portal, tooltip } from '@typie/ui/actions';
  import { Icon, RingSpinner } from '@typie/ui/components';
  import { getThemeContext } from '@typie/ui/context';
  import { Toast } from '@typie/ui/notification';
  import { clamp, debounce, throttle } from '@typie/ui/utils';
  import dayjs from 'dayjs';
  import mixpanel from 'mixpanel-browser';
  import { onMount } from 'svelte';
  import { on } from 'svelte/events';
  import { fly } from 'svelte/transition';
  import ClockRewindIcon from '~icons/lucide/clock-arrow-up';
  import IconClockFading from '~icons/lucide/clock-fading';
  import MinusIcon from '~icons/lucide/minus';
  import PlusIcon from '~icons/lucide/plus';
  import { Img } from '$lib/components';
  import { Editor, getEditorContext } from '$lib/editor-ffi/editor.svelte';
  import { graphql } from '$mearie';
  import { getPane, getPaneGroup } from '../../@pane/context.svelte';
  import { getDocumentPanelFocusReturn } from './focus-return.svelte';
  import type { Action } from 'svelte/action';
  import type { PointerEventHandler } from 'svelte/elements';
  import type { DocumentPanelV2Timeline_document$key } from '$mearie';

  type Props = {
    document$key: DocumentPanelV2Timeline_document$key;
  };

  let { document$key }: Props = $props();

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
            characterCount

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

  const ctx = getEditorContext();
  const theme = getThemeContext();
  const pane = getPane();
  const paneGroup = getPaneGroup();
  const focusReturn = getDocumentPanelFocusReturn();

  const editorContainer = $derived(ctx.editor?.scrollContainerEl);

  let timelineEditor: Editor | undefined;
  let creatingTimeline = false;
  let pendingHeadId: string | null = null;
  let destroyed = false;

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

  const exitTimeline = (): void => {
    ctx.editor = ctx.liveEditor;
    creatingTimeline = false;
    pendingHeadId = null;
    try {
      timelineEditor?.destroy();
    } finally {
      timelineEditor = undefined;
    }
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

  $effect(() => {
    if (query.data && selectedHeadId === null && headsAsc.length > 0) {
      // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
      selectedHeadId = headsAsc.at(-1)!.id;
    }
  });

  const scrollToHead = debounce((headId: string) => {
    const element = globalThis.document.querySelector(`[data-panel-timeline-head="${headId}"]`) as HTMLElement;
    element?.scrollIntoView({ behavior: 'smooth', block: 'center' });
  }, 50);

  const applyHead = async (headId: string): Promise<void> => {
    const liveEditor = ctx.liveEditor;
    if (!liveEditor) return;
    const head = headsAsc.find((h) => h.id === headId);
    if (!head) return;

    let plain;
    try {
      plain = liveEditor.materializeAt(Uint8Array.fromBase64(head.heads), [...(query.data?.document.sweepTombstones ?? [])]);
    } catch {
      if (selectedHeadId === headId && shownHeadId !== null) selectedHeadId = shownHeadId;
      return;
    }

    if (timelineEditor) {
      timelineEditor.setDoc(plain);
      shownHeadId = headId;
      return;
    }

    if (creatingTimeline) {
      pendingHeadId = headId;
      return;
    }

    creatingTimeline = true;
    try {
      const created = await Editor.createFromDoc(plain, liveEditor.viewport, theme.currentThemeVariant);
      created.readOnly = true;
      if (destroyed) {
        created.destroy();
        return;
      }
      timelineEditor = created;
      ctx.editor = created;
      shownHeadId = headId;
    } finally {
      creatingTimeline = false;
    }

    if (pendingHeadId !== null) {
      const next = pendingHeadId;
      pendingHeadId = null;
      void applyHead(next);
    }
  };
  const updateView = throttle(applyHead, 32);

  let prevSelectedId: string | null = null;
  $effect(() => {
    const currentId = selectedHeadId;
    if (!currentId || !query.data) return;
    if (currentId === prevSelectedId) return;
    prevSelectedId = currentId;
    scrollToHead.call(currentId);
    if (isDraggingSlider) {
      updateView.call(currentId);
    } else {
      updateView.cancel();
      void applyHead(currentId);
    }
  });

  const handleSlide: PointerEventHandler<HTMLElement> = (e) => {
    if (!e.currentTarget.parentElement || headsAsc.length === 0) return;
    const { left: parentLeft, width: parentWidth } = e.currentTarget.parentElement.getBoundingClientRect();
    const ratio = clamp((e.clientX - parentLeft) / parentWidth, 0, 1);
    const index = Math.round(ratio * max);
    if (Object.hasOwn(headsAsc, index)) selectedHeadId = headsAsc[index].id;
  };

  const restore = async () => {
    const head = shownHead;
    if (!head) return;

    try {
      await revertDocument({ input: { documentId: document.data.id, headId: head.id } });
    } catch {
      Toast.error('복원에 실패했어요. 잠시 후 다시 시도해 주세요.');
      return;
    }

    exitTimeline();
    paneGroup.state.current.panelExpandedByPaneId[pane.id] = false;
    focusReturn.restore();
    Toast.success(`${dayjs(head.updatedAt).formatAsSmart()} 시점으로 복원되었습니다`);
    mixpanel.track('document_timeline_restore');
  };

  const slider: Action<HTMLElement> = (element) => {
    $effect(() => {
      const dragstart = on(element, 'dragstart', (e) => e.preventDefault());
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
        if (selectedHeadId) {
          updateView.cancel();
          void applyHead(selectedHeadId);
        }
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
      alignItems: 'center',
      height: '41px',
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
    {:else if headsAsc.length === 0}
      <div class={center({ padding: '32px', flexDirection: 'column', gap: '8px', color: 'text.subtle', fontSize: '13px' })}>
        <Icon style={css.raw({ color: 'text.faint' })} icon={IconClockFading} size={24} />
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

            {#each group.heads as head (head.id)}
              {@const isSelected = selectedHeadId === head.id}
              {@const time = dayjs(head.updatedAt)}
              {@const headIndex = headsAsc.findIndex((h) => h.id === head.id)}
              {@const prevHead = headIndex > 0 ? headsAsc[headIndex - 1] : null}
              {@const charDiff = prevHead ? head.characterCount - prevHead.characterCount : head.characterCount}
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
                  _hover: { backgroundColor: isSelected ? 'surface.muted' : 'surface.subtle' },
                })}
                data-panel-timeline-head={head.id}
                onclick={() => {
                  selectedHeadId = head.id;
                }}
                type="button"
              >
                <Icon
                  style={css.raw({ flexShrink: '0', color: isSelected ? 'accent.brand.default' : 'text.subtle' })}
                  icon={ClockRewindIcon}
                  size={14}
                />

                <div class={flex({ flexDirection: 'column', align: 'start', gap: '2px', flex: '1' })}>
                  <div class={flex({ alignItems: 'center', gap: '8px' })}>
                    <div class={css({ fontSize: '13px', fontWeight: isSelected ? 'medium' : 'normal', color: 'text.default' })}>
                      {time.format('H시 mm분 ss초')}
                    </div>
                    {#if charDiff !== 0}
                      <div class={center()} in:fly={{ y: 10, duration: 150 }}>
                        <Icon
                          style={css.raw({ size: '10px', color: charDiff > 0 ? 'text.success' : 'text.danger' })}
                          icon={charDiff > 0 ? PlusIcon : MinusIcon}
                        />
                        <span class={css({ fontSize: '11px', fontWeight: 'medium', color: charDiff > 0 ? 'text.success' : 'text.danger' })}>
                          {Math.abs(charDiff).toLocaleString()}
                        </span>
                      </div>
                    {/if}
                  </div>
                  <div class={flex({ alignItems: 'center', gap: '6px' })}>
                    <div class={css({ fontSize: '11px', color: 'text.subtle' })}>
                      {time.fromNow()}
                    </div>
                    {#if head.contributors.length > 0}
                      <div class={flex({ alignItems: 'center' })}>
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
            {/each}
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>

{#if editorContainer && !isLoading && headsAsc.length > 0}
  <div
    class={center({ position: 'absolute', left: '0', right: '0', bottom: '32px', pointerEvents: 'none' })}
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
        pointerEvents: 'auto',
      })}
    >
      <Icon style={css.raw({ color: 'gray.500' })} icon={IconClockFading} size={18} />

      <div class={flex({ position: 'relative', flexGrow: '1', align: 'center', minWidth: '100px', maxWidth: '420px', height: '36px' })}>
        <button
          class={cx('group', css({ position: 'relative', width: 'full', height: '16px', overflow: 'hidden', cursor: 'pointer' }))}
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
              _groupHover: { backgroundColor: { base: 'gray.400', _dark: 'dark.gray.600' } },
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
              _hover: { scale: '[1.2]', boxShadow: '[0 4px 12px rgba(0,0,0,0.15)]' },
              _active: { scale: '[1.1]' },
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
        {dayjs(shownHead?.updatedAt).formatAsSmart()}
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
        {dayjs(headsAsc[sliderIndex]?.updatedAt).formatAsSmart()}
        <div class={css({ size: '6px', backgroundColor: 'surface.dark', zIndex: 'overEditor' })} use:arrow></div>
      </div>

      {#if shownHeadId === null || shownHeadId === latestHeadId}
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
            backgroundColor: 'accent.brand.default',
            color: 'text.bright',
            borderRadius: '8px',
            fontSize: '13px',
            fontWeight: 'medium',
            cursor: 'pointer',
            transition: 'all',
            transitionDuration: '150ms',
            _hover: { backgroundColor: 'accent.brand.hover', transform: 'translateY(-1px)' },
            _active: { backgroundColor: 'accent.brand.active', transform: 'translateY(0)' },
          })}
          onclick={restore}
          type="button"
          use:tooltip={{ message: '이 시점으로 문서를 복원하고 타임라인에 새로 추가합니다', placement: 'top' }}
        >
          <Icon style={css.raw({ size: '14px' })} icon={IconClockFading} />
          복원
        </button>
      {/if}
    </div>
  </div>
{/if}
