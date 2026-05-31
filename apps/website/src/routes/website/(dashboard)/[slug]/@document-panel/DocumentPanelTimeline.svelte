<script lang="ts">
  import { createFragment, createQuery } from '@mearie/svelte';
  import { css, cx } from '@typie/styled-system/css';
  import { center, flex, wrap } from '@typie/styled-system/patterns';
  import { createFloatingActions, infiniteScroll, portal, tooltip } from '@typie/ui/actions';
  import { Icon, RingSpinner } from '@typie/ui/components';
  import { Toast } from '@typie/ui/notification';
  import { clamp, debounce, throttle } from '@typie/ui/utils';
  import dayjs from 'dayjs';
  import mixpanel from 'mixpanel-browser';
  import { onMount, tick, untrack } from 'svelte';
  import { on } from 'svelte/events';
  import { SvelteMap } from 'svelte/reactivity';
  import { fly } from 'svelte/transition';
  import ClockRewindIcon from '~icons/lucide/clock-arrow-up';
  import IconClockFading from '~icons/lucide/clock-fading';
  import MinusIcon from '~icons/lucide/minus';
  import PlusIcon from '~icons/lucide/plus';
  import { idleCallback } from '$lib/editor/utils';
  import { graphql } from '$mearie';
  import { getPane, getPaneGroup } from '../@pane/context.svelte';
  import type { Action } from 'svelte/action';
  import type { PointerEventHandler } from 'svelte/elements';
  import type { Editor } from '$lib/editor/editor.svelte';
  import type { DocumentPanelTimeline_document$key } from '$mearie';

  type Props = {
    document$key: DocumentPanelTimeline_document$key;
    editor: Editor;
  };

  let { document$key, editor }: Props = $props();

  const document = createFragment(
    graphql(`
      fragment DocumentPanelTimeline_document on Document {
        id

        entity {
          id
          slug
        }
      }
    `),
    () => document$key,
  );

  let queryVars = $state<{ slug: string; first: number; before?: string | null } | null>(null);

  const query = createQuery(
    graphql(`
      query Editor_DocumentPanelTimeline_Query($slug: String!, $first: Int!, $before: DateTime) {
        document(slug: $slug) {
          id

          versionMetas {
            id
            createdAt
          }

          versions(first: $first, before: $before) {
            id
            version
            createdAt
          }
        }
      }
    `),
    () => queryVars ?? { slug: '', first: 20 },
    () => ({ skip: !queryVars }),
  );

  const pane = getPane();
  const paneGroup = getPaneGroup();

  let editorContainer = $derived.by(() => editor.scrollContainerEl);

  let selectedVersionId = $state<string | null>(null);
  let isLoading = $state(true);
  let versionCharCounts = $state<Record<string, number>>({});
  let wasDetached = $state(false);

  const versionMetas = $derived(query.data?.document.versionMetas ?? []);
  let versionCache = new SvelteMap<string, string>();

  const loadedVersions = $derived.by(() => {
    return versionMetas.filter((meta) => versionCache.has(meta.id)).map((meta) => ({ ...meta, version: versionCache.get(meta.id) ?? '' }));
  });

  const groupedVersions = $derived.by(() => {
    const groups: { date: string; versions: typeof loadedVersions }[] = [];
    const dateGroups: Record<string, typeof loadedVersions> = {};

    loadedVersions.forEach((version) => {
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

  const sliderIndex = $derived(selectedVersionId ? versionMetas.findIndex((s) => s.id === selectedVersionId) : versionMetas.length - 1);
  const max = $derived(versionMetas.length > 0 ? versionMetas.length - 1 : 0);
  const p = $derived(max > 0 && sliderIndex >= 0 ? `${(sliderIndex / max) * 100}%` : '100%');

  const processVersionsCharacterCounts = (versions: { id: string; version: string }[]) => {
    if (versions.length === 0) return;

    let currentIndex = 0;

    const processNext = () => {
      if (currentIndex < versions.length) {
        const version = versions[currentIndex];
        if (versionCharCounts[version.id] === undefined) {
          const versionData = Uint8Array.fromBase64(version.version);
          const count = editor.getCharacterCountAtVersion(versionData);
          if (count !== undefined) {
            versionCharCounts = { ...versionCharCounts, [version.id]: count };
          }
        }
        currentIndex++;

        if (currentIndex < versions.length) {
          idleCallback(processNext);
        }
      }
    };

    idleCallback(processNext);
  };

  let isLoadingMore = $state(false);
  let loadingPromise: Promise<void> | null = null;
  let hasMoreVersions = $derived(versionMetas.some((meta) => !versionCache.has(meta.id)));
  // TODO: query.load() returned a promise with result data. In Mearie's createQuery, queries are reactive.
  // These functions use a workaround: set queryVars to trigger a fetch, then watch query.data reactively.
  // For now, we set variables and use $effect to process results, which may need refinement.

  let pendingLoadResolve: ((data: typeof query.data) => void) | null = null;

  const loadQuery = (vars: { slug: string; first: number; before?: string | null }): Promise<NonNullable<typeof query.data>> => {
    return new Promise((resolve) => {
      pendingLoadResolve = resolve as (data: typeof query.data) => void;
      queryVars = { ...vars };
      query.refetch();
    });
  };

  $effect(() => {
    if (queryVars && query.data && !query.loading && pendingLoadResolve) {
      const resolve = pendingLoadResolve;
      pendingLoadResolve = null;
      resolve(query.data);
    }
  });

  const loadMoreVersions = async () => {
    if (loadingPromise) {
      await loadingPromise;
      return;
    }
    if (!query.data || loadedVersions.length === 0) return;
    if (!hasMoreVersions) return;

    isLoadingMore = true;
    loadingPromise = (async () => {
      try {
        const oldestDisplayed = loadedVersions[0];
        if (!oldestDisplayed) return;

        const result = await loadQuery({
          slug: document.data.entity.slug,
          first: 20,
          before: oldestDisplayed.createdAt,
        });

        const newVersions: { id: string; version: string }[] = [];
        for (const v of result.document.versions) {
          if (!versionCache.has(v.id)) {
            versionCache.set(v.id, v.version);
            newVersions.push(v);
          }
        }

        processVersionsCharacterCounts(newVersions);
      } finally {
        isLoadingMore = false;
        loadingPromise = null;
      }
    })();

    await loadingPromise;
  };

  const initialize = async () => {
    isLoading = true;
    try {
      const result = await loadQuery({ slug: document.data.entity.slug, first: 20, before: null });

      const newCache = new SvelteMap<string, string>();
      for (const v of result.document.versions) {
        newCache.set(v.id, v.version);
      }
      versionCache = newCache;

      const metas = result.document.versionMetas;
      if (metas.length > 0) {
        // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
        selectedVersionId = metas.at(-1)!.id;
      }

      processVersionsCharacterCounts([...result.document.versions] as { id: string; version: string }[]);
    } finally {
      tick().then(() => {
        isLoading = false;
      });
    }
  };

  onMount(() => {
    wasDetached = editor.isDetached();
    const wasReadOnly = editor.isReadOnly();
    editor.setReadOnly(true);
    initialize();

    return () => {
      scrollToVersion.cancel();
      updateViewVersion.cancel();
      editor.setReadOnly(wasReadOnly);
      if (editor.isDetached() && !wasDetached) {
        editor.checkoutToLatest();
      }
    };
  });

  const scrollToVersion = debounce((versionId: string) => {
    const element = globalThis.document.querySelector(`[data-panel-timeline-version="${versionId}"]`) as HTMLElement;
    element?.scrollIntoView({ behavior: 'smooth', block: 'center' });
  }, 50);

  let prevSelectedId: string | null = null;

  $effect(() => {
    const currentId = selectedVersionId;
    if (!currentId || !query.data) return;

    if (currentId === prevSelectedId) return;
    prevSelectedId = currentId;

    untrack(async () => {
      while (!versionCache.has(currentId) && hasMoreVersions) {
        await loadMoreVersions();
      }

      scrollToVersion.call(currentId);
      updateViewVersion.call(currentId);
    });
  });

  const updateViewVersion = throttle((versionId: string) => {
    const versionBinary = versionCache.get(versionId);
    if (!versionBinary) return;

    const versionData = Uint8Array.fromBase64(versionBinary);
    editor.checkout(versionData);
  }, 32);

  const handleSlide: PointerEventHandler<HTMLElement> = (e) => {
    if (!e.currentTarget.parentElement || versionMetas.length === 0) {
      return;
    }

    const { left: parentLeft, width: parentWidth } = e.currentTarget.parentElement.getBoundingClientRect();
    const { clientX: pointerLeft } = e;
    const ratio = clamp((pointerLeft - parentLeft) / parentWidth, 0, 1);
    const index = Math.round(ratio * max);

    if (versionMetas[index]) {
      selectedVersionId = versionMetas[index].id;
    }
  };

  const restore = () => {
    const version = versionMetas[sliderIndex];
    if (!version) return;

    const versionBinary = versionCache.get(version.id);
    if (!versionBinary) return;

    const versionData = Uint8Array.fromBase64(versionBinary);

    editor.checkoutToLatest();
    editor.revertTo(versionData);

    paneGroup.state.current.panelExpandedByPaneId[pane.id] = false;
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
    {:else if versionMetas.length === 0}
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
              {@const versionIndex = loadedVersions.findIndex((s) => s.id === version.id)}
              {@const prevVersion = versionIndex > 0 ? loadedVersions[versionIndex - 1] : null}
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
                    const currentIndex = loadedVersions.findIndex((s) => s.id === version.id);
                    const nextVersion = loadedVersions[currentIndex + 1];
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
                    const currentIndex = loadedVersions.findIndex((s) => s.id === version.id);
                    const prevVersion = loadedVersions[currentIndex - 1];
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

        {#if hasMoreVersions}
          <div class={center({ padding: '16px' })} use:infiniteScroll={{ onLoadMore: loadMoreVersions, enabled: !isLoadingMore }}>
            {#if isLoadingMore}
              <RingSpinner style={css.raw({ size: '20px', color: 'text.subtle' })} />
            {:else}
              <span class={css({ fontSize: '12px', color: 'text.subtle' })}>스크롤하여 더 불러오기</span>
            {/if}
          </div>
        {/if}
      </div>
    {/if}
  </div>
</div>

{#if editorContainer && !isLoading && versionMetas.length > 0}
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
        {dayjs(versionMetas[sliderIndex]?.createdAt).formatAsSmart()}
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
        {dayjs(versionMetas[sliderIndex]?.createdAt).formatAsSmart()}
        <div
          class={css({
            size: '6px',
            backgroundColor: 'surface.dark',
            zIndex: 'overEditor',
          })}
          use:arrow
        ></div>
      </div>

      {#if selectedVersionId === versionMetas.at(-1)?.id}
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
