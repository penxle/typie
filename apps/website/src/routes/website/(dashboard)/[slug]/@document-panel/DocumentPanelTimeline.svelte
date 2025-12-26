<script lang="ts">
  import { css, cx } from '@typie/styled-system/css';
  import { center, flex, wrap } from '@typie/styled-system/patterns';
  import { createFloatingActions, portal, tooltip } from '@typie/ui/actions';
  import { Icon, RingSpinner } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import { Toast } from '@typie/ui/notification';
  import { clamp, debounce, throttle } from '@typie/ui/utils';
  import dayjs from 'dayjs';
  import mixpanel from 'mixpanel-browser';
  import { onMount, tick } from 'svelte';
  import { on } from 'svelte/events';
  import { fly } from 'svelte/transition';
  import ClockRewindIcon from '~icons/lucide/clock-arrow-up';
  import IconClockFading from '~icons/lucide/clock-fading';
  import MinusIcon from '~icons/lucide/minus';
  import PlusIcon from '~icons/lucide/plus';
  import { fragment, graphql } from '$graphql';
  import { findScroller, idleCallback } from '$lib/editor/utils';
  import { getViewContext } from '../@split-view/context.svelte';
  import type { Action } from 'svelte/action';
  import type { PointerEventHandler } from 'svelte/elements';
  import type { DocumentPanelTimeline_document } from '$graphql';
  import type { Editor } from '$lib/editor/editor.svelte';

  type Props = {
    $document: DocumentPanelTimeline_document;
    editor: Editor;
  };

  let { $document: _document, editor }: Props = $props();

  const document = fragment(
    _document,
    graphql(`
      fragment DocumentPanelTimeline_document on Document {
        id

        entity {
          id
          slug
        }
      }
    `),
  );

  const query = graphql(`
    query Editor_DocumentPanelTimeline_Query($slug: String!) @client {
      document(slug: $slug) {
        id

        versions {
          id
          version
          createdAt
        }
      }
    }
  `);

  const app = getAppContext();
  const view = getViewContext();

  let editorContainer = $state<HTMLElement | null>(null);

  $effect(() => {
    const containerEl = editor.extensionArea.containerEl;
    if (containerEl) {
      editorContainer = findScroller(containerEl).parentElement;
    }
  });

  let selectedVersionId = $state<string | null>(null);
  let isLoading = $state(true);
  let versionCharCounts = $state<Record<string, number>>({});
  let wasDetached = $state(false);
  let versions = $state<{ id: string; version: string; createdAt: string }[]>([]);

  const groupedVersions = $derived.by(() => {
    const groups: { date: string; versions: typeof versions }[] = [];
    const dateGroups: Record<string, typeof versions> = {};

    versions.forEach((version) => {
      const date = dayjs(version.createdAt).format('YYYY년 M월 D일');
      if (!dateGroups[date]) {
        dateGroups[date] = [];
      }
      dateGroups[date].unshift(version);
    });

    Object.entries(dateGroups).forEach(([date, versionList]) => {
      groups.push({ date, versions: versionList });
    });

    return groups.toSorted((a, b) => dayjs(b.versions[0].createdAt).valueOf() - dayjs(a.versions[0].createdAt).valueOf());
  });

  const { anchor, floating, arrow } = createFloatingActions({
    placement: 'top',
    offset: 8,
    arrow: true,
  });

  let showTooltip = $state(false);
  let isDraggingSlider = $state(false);

  const sliderIndex = $derived(selectedVersionId ? versions.findIndex((s) => s.id === selectedVersionId) : versions.length - 1);
  const max = $derived(versions.length > 0 ? versions.length - 1 : 0);
  const p = $derived(max > 0 && sliderIndex >= 0 ? `${(sliderIndex / max) * 100}%` : '100%');

  const initialize = async () => {
    isLoading = true;
    try {
      const result = await query.load({ slug: $document.entity.slug });
      versions = [...result.document.versions];

      if (versions.length > 0) {
        const latestVersion = versions.at(-1);
        if (latestVersion) {
          selectedVersionId = latestVersion.id;
        }
      }

      const counts: Record<string, number> = {};
      let currentIndex = 0;

      const processNextVersion = () => {
        if (currentIndex < versions.length) {
          const version = versions[currentIndex];
          const versionData = Uint8Array.fromBase64(version.version);
          const count = editor.getCharacterCountAtVersion(versionData);
          if (count !== undefined) {
            counts[version.id] = count;
          }
          currentIndex++;
          versionCharCounts = { ...counts };

          if (currentIndex < versions.length) {
            idleCallback(processNextVersion);
          }
        }
      };

      idleCallback(processNextVersion);
    } finally {
      tick().then(() => {
        isLoading = false;
      });
    }
  };

  onMount(() => {
    wasDetached = editor.isDetached();
    initialize();

    return () => {
      if (editor.isDetached() && !wasDetached) {
        editor.checkoutToLatest();
      }
    };
  });

  const scrollToVersion = debounce((versionId: string) => {
    const element = globalThis.document.querySelector(`[data-panel-timeline-version="${versionId}"]`) as HTMLElement;
    element?.scrollIntoView({ behavior: 'smooth', block: 'center' });
  }, 50);

  $effect(() => {
    if (selectedVersionId && versions.length > 0) {
      scrollToVersion(selectedVersionId);
      updateViewVersion.call(selectedVersionId);
    }
  });

  const updateViewVersion = throttle((versionId: string) => {
    const version = versions.find((s) => s.id === versionId);
    if (!version) return;

    const versionData = Uint8Array.fromBase64(version.version);
    editor.checkout(versionData);
  }, 32);

  const handleSlide: PointerEventHandler<HTMLElement> = (e) => {
    if (!e.currentTarget.parentElement || versions.length === 0) {
      return;
    }

    const { left: parentLeft, width: parentWidth } = e.currentTarget.parentElement.getBoundingClientRect();
    const { clientX: pointerLeft } = e;
    const ratio = clamp((pointerLeft - parentLeft) / parentWidth, 0, 1);
    const index = Math.round(ratio * max);

    if (versions[index]) {
      selectedVersionId = versions[index].id;
    }
  };

  const restore = () => {
    const version = versions[sliderIndex];
    if (!version) return;

    const versionData = Uint8Array.fromBase64(version.version);

    editor.checkoutToLatest();
    editor.revertTo(versionData);

    app.preference.current.panelExpandedByViewId[view.id] = false;
    Toast.success(`${dayjs(version.createdAt).formatAsSmart()} 시점으로 복원되었습니다`);
    mixpanel.track('document_timeline_restore');
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
    {:else if versions.length === 0}
      <div class={center({ padding: '32px', flexDirection: 'column', gap: '8px', color: 'text.subtle', fontSize: '13px' })}>
        <Icon style={css.raw({ color: 'text.faint' })} icon={IconClockFading} size={24} />
        아직 버전 기록이 없습니다
      </div>
    {:else}
      <div class={flex({ flexDirection: 'column' })}>
        {#each groupedVersions as group (group.date)}
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

            {#each group.versions as version (version.id)}
              {@const isSelected = selectedVersionId === version.id}
              {@const time = dayjs(version.createdAt)}
              {@const currentCount = versionCharCounts[version.id] ?? null}
              {@const versionIndex = versions.findIndex((s) => s.id === version.id)}
              {@const prevVersion = versionIndex > 0 ? versions[versionIndex - 1] : null}
              {@const prevCount = prevVersion && versionCharCounts[prevVersion.id] !== undefined ? versionCharCounts[prevVersion.id] : null}
              {@const charDiff =
                versionIndex === 0 ? currentCount : prevCount === null || currentCount === null ? null : currentCount - prevCount}
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
                data-panel-timeline-version={version.id}
                onclick={() => {
                  selectedVersionId = version.id;
                }}
                onkeydown={(e) => {
                  if (e.key === 'Enter' || e.key === ' ') {
                    e.preventDefault();
                    selectedVersionId = version.id;
                  } else if (e.key === 'ArrowUp') {
                    e.preventDefault();
                    const currentIndex = versions.findIndex((s) => s.id === version.id);
                    const nextVersion = versions[currentIndex + 1];
                    if (nextVersion) {
                      selectedVersionId = nextVersion.id;
                      tick().then(() => {
                        const nextElement = globalThis.document.querySelector(
                          `[data-panel-timeline-version="${nextVersion.id}"]`,
                        ) as HTMLElement;
                        nextElement?.focus();
                      });
                    }
                  } else if (e.key === 'ArrowDown') {
                    e.preventDefault();
                    const currentIndex = versions.findIndex((s) => s.id === version.id);
                    const prevVersion = versions[currentIndex - 1];
                    if (prevVersion) {
                      selectedVersionId = prevVersion.id;
                      tick().then(() => {
                        const prevElement = globalThis.document.querySelector(
                          `[data-panel-timeline-version="${prevVersion.id}"]`,
                        ) as HTMLElement;
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

{#if editorContainer && !isLoading && versions.length > 0}
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
        {dayjs(versions[sliderIndex]?.createdAt).formatAsSmart()}
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
        {dayjs(versions[sliderIndex]?.createdAt).formatAsSmart()}
        <div
          class={css({
            size: '6px',
            backgroundColor: 'surface.dark',
            zIndex: 'overEditor',
          })}
          use:arrow
        ></div>
      </div>

      {#if selectedVersionId === versions.at(-1)?.id}
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
            message: '이 시점으로 문서를 복원하고 타임라인에 새로 추가합니다',
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
