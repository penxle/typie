<script lang="ts">
  import { createMutation, createQuery } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { pointerCapture, tooltip } from '@typie/ui/actions';
  import { Icon, Popover, TextInput } from '@typie/ui/components';
  import { Toast } from '@typie/ui/notification';
  import dayjs from 'dayjs';
  import mixpanel from 'mixpanel-browser';
  import GripVerticalIcon from '~icons/lucide/grip-vertical';
  import PinOffIcon from '~icons/lucide/pin-off';
  import PlusIcon from '~icons/lucide/plus';
  import SearchIcon from '~icons/lucide/search';
  import { SettingsCard, SettingsDivider } from '$lib/components';
  import { cache } from '$lib/graphql';
  import { publicationErrorCode } from '$lib/publication/error';
  import { publicationErrorMessage } from '$lib/publication/publish-form';
  import { graphql } from '$mearie';
  import { SubscribeModal } from '../@subscription/subscribe-modal.svelte';
  import DragIndicator from '../@tree/DragIndicator.svelte';
  import { pinnedPlacementIndicator, resolvePinnedOrders, resolvePinnedPlacementAt } from '../@tree/pinned-placement';
  import type { Action } from 'svelte/action';
  import type { DragIndicatorState } from '../@tree/DragIndicator.svelte';

  const PIN_LIMIT = 3;
  const SEARCH_DEBOUNCE_MS = 300;
  const DRAG_MOVE_THRESHOLD_PX = 10;

  type Props = {
    siteId: string;
  };

  let { siteId }: Props = $props();

  const pinnedQuery = createQuery(
    graphql(`
      query DashboardLayout_SiteSettingsModal_PinnedPublicationsEditor_Query($siteId: ID!) {
        site(siteId: $siteId) {
          id

          view {
            id

            pinnedPublications {
              id
              title
              pinnedOrder
              publishedAt

              folder {
                id
                name
              }
            }

            recentPublications(first: 10) {
              publications {
                id
                title
                publishedAt

                folder {
                  id
                  name
                }
              }
            }
          }
        }
      }
    `),
    () => ({ siteId }),
  );

  const [pinPublication] = createMutation(
    graphql(`
      mutation DashboardLayout_SiteSettingsModal_PinnedPublicationsEditor_PinPublication_Mutation($input: PinPublicationInput!) {
        pinPublication(input: $input) {
          id
          pinnedOrder
        }
      }
    `),
  );

  const [unpinPublication] = createMutation(
    graphql(`
      mutation DashboardLayout_SiteSettingsModal_PinnedPublicationsEditor_UnpinPublication_Mutation($input: UnpinPublicationInput!) {
        unpinPublication(input: $input) {
          id
          pinnedOrder
        }
      }
    `),
  );

  let query = $state('');
  let debouncedQuery = $state('');
  let debounceTimer: ReturnType<typeof setTimeout> | undefined;
  let pickerOpen = $state(false);

  const searchQuery = createQuery(
    graphql(`
      query DashboardLayout_SiteSettingsModal_PinnedPublicationsEditor_Search_Query($query: String!, $siteId: ID!) {
        search(query: $query, siteId: $siteId) {
          hits {
            __typename

            ... on SearchHitDocument {
              document {
                id
                title

                publication {
                  id
                  state
                  publishedAt
                }
              }
            }
          }
        }
      }
    `),
    () => ({ query: debouncedQuery, siteId }),
    () => ({ skip: !pickerOpen || !debouncedQuery }),
  );

  $effect(() => {
    return () => {
      if (debounceTimer) clearTimeout(debounceTimer);
    };
  });

  const scheduleSearch = () => {
    if (debounceTimer) clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => {
      debouncedQuery = query.trim();
    }, SEARCH_DEBOUNCE_MS);
  };

  const resetSearch = () => {
    if (debounceTimer) clearTimeout(debounceTimer);
    query = '';
    debouncedQuery = '';
  };

  const view = $derived(pinnedQuery.data?.site.view);
  const pinned = $derived(view?.pinnedPublications ?? []);
  const pinnedIds = $derived(new Set(pinned.map((publication) => publication.id)));
  const full = $derived(pinned.length >= PIN_LIMIT);
  const orderedItems = $derived(pinned.map((publication) => ({ id: publication.id, pinnedOrder: publication.pinnedOrder ?? '' })));

  type Candidate = { id: string; title: string; publishedAt: string; folderName: string | null };

  const candidates = $derived.by((): Candidate[] => {
    if (debouncedQuery) {
      return (searchQuery.data?.search.hits ?? [])
        .map((hit): Candidate | null => {
          if (hit.__typename !== 'SearchHitDocument') return null;
          const publication = hit.document.publication;
          if (!publication || publication.state !== 'PUBLISHED' || !publication.publishedAt) return null;
          return { id: publication.id, title: hit.document.title, publishedAt: publication.publishedAt, folderName: null };
        })
        .filter((candidate): candidate is Candidate => candidate !== null)
        .filter((candidate) => !pinnedIds.has(candidate.id));
    }

    return (view?.recentPublications.publications ?? [])
      .filter((publication) => !pinnedIds.has(publication.id))
      .map((publication) => ({
        id: publication.id,
        title: publication.title,
        publishedAt: publication.publishedAt,
        folderName: publication.folder?.name ?? null,
      }));
  });

  const emptyMessage = $derived(
    debouncedQuery ? (searchQuery.loading ? '검색 중...' : '찾는 글이 없어요.') : '고정할 수 있는 글이 없어요.',
  );

  const formatDate = (value: string) => dayjs(value).format('YYYY. M. D.');
  const metaOf = (folderName: string | null, publishedAt: string) =>
    folderName ? `${folderName} · ${formatDate(publishedAt)}` : formatDate(publishedAt);

  const invalidate = () => {
    cache.invalidate({ __typename: 'SiteView', id: siteId, $field: 'pinnedPublications' });
    cache.invalidate({ __typename: 'SiteView', id: siteId, $field: 'recentPublications' });
    void pinnedQuery.refetch();
  };

  let running = $state(false);

  const run = async (action: () => Promise<unknown>) => {
    if (running) return false;
    if (!SubscribeModal.gate('site_settings')) return false;

    running = true;
    try {
      await action();
      invalidate();
      Toast.success('스페이스 설정이 업데이트됐어요.');
      return true;
    } catch (err) {
      Toast.error(publicationErrorMessage(publicationErrorCode(err)));
      return false;
    } finally {
      running = false;
    }
  };

  const pin = async (publicationId: string, close: () => void) => {
    const ok = await run(async () => {
      await pinPublication({ input: { publicationId } });
      mixpanel.track('pin_publication', { via: 'site_settings' });
    });
    if (ok && pinned.length + 1 >= PIN_LIMIT) close();
  };

  const unpin = async (publicationId: string) => {
    await run(async () => {
      await unpinPublication({ input: { publicationId } });
      mixpanel.track('unpin_publication', { via: 'site_settings' });
    });
  };

  const reorder = async (publicationId: string, orders: { lowerOrder: string | null; upperOrder: string | null }) => {
    await run(async () => {
      await pinPublication({ input: { publicationId, ...orders } });
      mixpanel.track('reorder_pinned_publication', { via: 'site_settings' });
    });
  };

  let listElement = $state<HTMLDivElement>();
  let dragging = $state<string | null>(null);
  let indicator = $state<DragIndicatorState>({});

  type DragSession = {
    id: string;
    startX: number;
    startY: number;
    active: boolean;
    orders: { lowerOrder: string | null; upperOrder: string | null } | null;
  };

  const finishDrag = () => {
    dragging = null;
    indicator = {};
  };

  const dragRow: Action<HTMLElement, string> = (element, id) => {
    const handle = pointerCapture<DragSession>(element, {
      start: (event) => {
        if (running || event.button !== 0 || !event.isPrimary) return null;
        return { id, startX: event.clientX, startY: event.clientY, active: false, orders: null };
      },
      move: (session, event) => {
        if (!session.active) {
          const distance = Math.abs(event.clientX - session.startX) + Math.abs(event.clientY - session.startY);
          if (distance <= DRAG_MOVE_THRESHOLD_PX) return;
          session.active = true;
          dragging = session.id;
        }

        const list = listElement;
        if (!list) return;

        const hit = document.elementFromPoint(event.clientX, event.clientY);
        const placement = resolvePinnedPlacementAt(list, hit, event.clientY);
        const orders = placement ? resolvePinnedOrders(orderedItems, [session.id], placement) : null;
        session.orders = orders;
        indicator = orders && placement ? pinnedPlacementIndicator(list, placement) : {};
      },
      end: (session) => {
        const orders = session.active ? session.orders : null;
        finishDrag();
        if (orders) void reorder(session.id, orders);
      },
      cancel: () => finishDrag(),
    });

    return { destroy: handle.destroy };
  };

  const iconButtonStyle = css.raw({
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    size: '28px',
    borderRadius: '6px',
    color: 'text.muted',
    transition: 'common',
    _hover: { backgroundColor: 'surface.hover', color: 'text.default' },
    _disabled: { opacity: '40' },
  });
</script>

<div>
  <div class={flex({ alignItems: 'center', justifyContent: 'space-between', marginBottom: '4px' })}>
    <h2 class={css({ fontSize: '16px', fontWeight: 'semibold', color: 'text.default' })}>
      고정 글
      <span
        class={css({ marginLeft: '2px', fontSize: '12px', fontWeight: 'medium', color: 'text.hint', fontVariantNumeric: 'tabular-nums' })}
      >
        {pinned.length}/{PIN_LIMIT}
      </span>
    </h2>

    <div use:tooltip={{ message: full ? '고정은 3개까지 할 수 있어요.' : '' }}>
      <Popover
        style={css.raw({
          display: 'flex',
          alignItems: 'center',
          gap: '6px',
          borderRadius: '6px',
          paddingX: '12px',
          paddingY: '6px',
          fontSize: '13px',
          fontWeight: 'medium',
          color: 'text.muted',
          transition: 'common',
          _hover: { backgroundColor: 'surface.hover' },
          _disabled: { opacity: '40' },
        })}
        contentStyle={css.raw({ width: '300px', padding: '4px' })}
        disabled={full}
        offset={4}
        onclose={resetSearch}
        placement="bottom-end"
        bind:open={pickerOpen}
      >
        {#snippet trigger()}
          <Icon style={css.raw({ color: 'text.muted' })} icon={PlusIcon} size={14} />
          <span>추가</span>
        {/snippet}

        {#snippet children({ close })}
          <div class={css({ padding: '4px' })}>
            <TextInput
              autofocus
              leftIcon={SearchIcon}
              oninput={scheduleSearch}
              placeholder="글 제목으로 찾기"
              size="sm"
              bind:value={query}
            />
          </div>

          <div
            class={css({
              paddingX: '8px',
              paddingTop: '8px',
              paddingBottom: '4px',
              fontSize: '11px',
              fontWeight: 'semibold',
              color: 'text.hint',
            })}
          >
            {debouncedQuery ? '검색 결과' : '최근 발행한 글'}
          </div>

          <div class={css({ maxHeight: '240px', overflowY: 'auto' })} aria-label="고정할 글" role="listbox">
            {#if candidates.length === 0}
              <div class={css({ paddingX: '8px', paddingY: '12px', fontSize: '12px', color: 'text.hint', textAlign: 'center' })}>
                {emptyMessage}
              </div>
            {:else}
              {#each candidates as candidate (candidate.id)}
                <button
                  class={flex({
                    alignItems: 'center',
                    gap: '8px',
                    width: 'full',
                    minHeight: '34px',
                    paddingX: '8px',
                    paddingY: '4px',
                    borderRadius: '6px',
                    fontSize: '13px',
                    textAlign: 'left',
                    transition: 'common',
                    _hover: { backgroundColor: 'surface.hover' },
                    _focusVisible: { backgroundColor: 'surface.hover', outline: 'none' },
                    _disabled: { opacity: '40' },
                  })}
                  aria-selected="false"
                  disabled={running}
                  onclick={() => pin(candidate.id, close)}
                  role="option"
                  type="button"
                >
                  <span class={css({ flex: '1', minWidth: '0', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' })}>
                    {candidate.title}
                  </span>
                  <span class={css({ flexShrink: '0', fontSize: '12px', color: 'text.hint', whiteSpace: 'nowrap' })}>
                    {metaOf(candidate.folderName, candidate.publishedAt)}
                  </span>
                </button>
              {/each}
            {/if}
          </div>
        {/snippet}
      </Popover>
    </div>
  </div>

  <p class={css({ fontSize: '13px', color: 'text.muted', lineHeight: '[1.6]', marginBottom: '20px' })}>
    스페이스 홈 맨 위에 보이는 글이에요. 위에서부터 홈에 보이는 순서예요.
  </p>

  <SettingsCard>
    {#if !view}
      <div class={css({ padding: '20px', fontSize: '13px', color: 'text.hint', textAlign: 'center' })}>불러오는 중...</div>
    {:else if pinned.length === 0}
      <div class={css({ padding: '20px', fontSize: '13px', color: 'text.hint', textAlign: 'center' })}>아직 고정한 글이 없어요.</div>
    {:else}
      <div bind:this={listElement}>
        {#each pinned as publication, index (publication.id)}
          {#if index > 0}
            <SettingsDivider />
          {/if}
          <div
            class={flex({
              alignItems: 'center',
              gap: '12px',
              minHeight: '56px',
              paddingLeft: '10px',
              paddingRight: '12px',
              paddingY: '10px',
              opacity: dragging === publication.id ? '40' : '100',
              transition: 'opacity',
            })}
            data-id={publication.id}
          >
            <button
              class={css({
                display: 'flex',
                alignItems: 'center',
                padding: '4px',
                borderRadius: '4px',
                color: 'text.hint',
                cursor: 'grab',
                touchAction: 'none',
                transition: 'common',
                _hover: { color: 'text.default' },
              })}
              aria-label="순서 바꾸기"
              type="button"
              use:dragRow={publication.id}
            >
              <Icon icon={GripVerticalIcon} size={14} />
            </button>

            <div class={css({ flex: '1', minWidth: '0' })}>
              <div
                class={css({
                  fontSize: '13px',
                  fontWeight: 'medium',
                  color: 'text.default',
                  overflow: 'hidden',
                  textOverflow: 'ellipsis',
                  whiteSpace: 'nowrap',
                })}
              >
                {publication.title}
              </div>
              <div class={css({ fontSize: '12px', color: 'text.hint', marginTop: '2px' })}>
                {metaOf(publication.folder?.name ?? null, publication.publishedAt)}
              </div>
            </div>

            <button
              class={css(iconButtonStyle)}
              aria-label="고정 해제"
              disabled={running}
              onclick={() => unpin(publication.id)}
              type="button"
              use:tooltip={{ message: '고정 해제', placement: 'top' }}
            >
              <Icon icon={PinOffIcon} size={14} />
            </button>
          </div>
        {/each}
      </div>
    {/if}
  </SettingsCard>
</div>

{#if dragging}
  <DragIndicator {indicator} />
{/if}
