<script lang="ts">
  import { createFragment, createMutation } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { Button, Icon, Menu, MenuItem } from '@typie/ui/components';
  import { Toast } from '@typie/ui/notification';
  import mixpanel from 'mixpanel-browser';
  import PinOffIcon from '~icons/lucide/pin-off';
  import PlusIcon from '~icons/lucide/plus';
  import { SettingsCard, SettingsDivider, SettingsRow } from '$lib/components';
  import { publicationErrorCode } from '$lib/publication/error';
  import { appendOrder } from '$lib/publication/neighbor-order';
  import { publicationErrorMessage } from '$lib/publication/publish-form';
  import { graphql } from '$mearie';
  import { SubscribeModal } from '../@subscription/subscribe-modal.svelte';
  import ReorderableList from './ReorderableList.svelte';
  import type { DashboardLayout_SiteSettingsModal_SpacePinnedSection_space$key } from '$mearie';

  const PIN_LIMIT = 3;

  type Props = {
    space$key: DashboardLayout_SiteSettingsModal_SpacePinnedSection_space$key;
  };

  let { space$key }: Props = $props();

  const space = createFragment(
    graphql(`
      fragment DashboardLayout_SiteSettingsModal_SpacePinnedSection_space on Space {
        id

        pinnedPublications {
          id
          title
          pinnedOrder
        }

        publications(states: [PUBLISHED]) {
          id
          title
          pinnedOrder
        }
      }
    `),
    () => space$key,
  );

  const [pinPublication, pinResult] = createMutation(
    graphql(`
      mutation DashboardLayout_SiteSettingsModal_SpacePinnedSection_PinPublication_Mutation($input: PinPublicationInput!) {
        pinPublication(input: $input) {
          id
          pinnedOrder

          space {
            id

            pinnedPublications {
              id
              title
              pinnedOrder
            }

            publications(states: [PUBLISHED]) {
              id
              title
              pinnedOrder
            }
          }
        }
      }
    `),
  );

  const [unpinPublication, unpinResult] = createMutation(
    graphql(`
      mutation DashboardLayout_SiteSettingsModal_SpacePinnedSection_UnpinPublication_Mutation($input: UnpinPublicationInput!) {
        unpinPublication(input: $input) {
          id
          pinnedOrder

          space {
            id

            pinnedPublications {
              id
              title
              pinnedOrder
            }

            publications(states: [PUBLISHED]) {
              id
              title
              pinnedOrder
            }
          }
        }
      }
    `),
  );

  const pinned = $derived(space.data.pinnedPublications);
  const items = $derived(pinned.map((publication) => ({ id: publication.id, order: publication.pinnedOrder ?? '' })));
  const candidates = $derived(space.data.publications.filter((publication) => publication.pinnedOrder === null));
  const busy = $derived(pinResult.loading || unpinResult.loading);
  const canAdd = $derived(pinned.length < PIN_LIMIT && candidates.length > 0);

  const guard = async (run: () => Promise<unknown>, event: string) => {
    if (busy || !SubscribeModal.gate('space_settings')) return false;

    try {
      await run();
      mixpanel.track(event);
      return true;
    } catch (err) {
      Toast.error(publicationErrorMessage(publicationErrorCode(err)));
      return false;
    }
  };

  const pin = (publicationId: string) =>
    guard(() => pinPublication({ input: { publicationId, ...appendOrder(items.map((item) => item.order)) } }), 'pin_publication');

  const reorder = (move: { id: string; lowerOrder: string | null; upperOrder: string | null }) =>
    guard(
      () => pinPublication({ input: { publicationId: move.id, lowerOrder: move.lowerOrder, upperOrder: move.upperOrder } }),
      'move_pinned_publication',
    );

  const unpin = (publicationId: string) => guard(() => unpinPublication({ input: { publicationId } }), 'unpin_publication');
</script>

<div class={css({ marginTop: '40px' })}>
  <h2 class={css({ fontSize: '16px', fontWeight: 'semibold', color: 'text.default', marginBottom: '24px' })}>고정 글</h2>

  <SettingsCard>
    <ReorderableList disabled={busy} {items} onmove={reorder}>
      {#snippet row(id)}
        {@const publication = pinned.find((candidate) => candidate.id === id)}

        {#if publication}
          <div
            class={flex({
              alignItems: 'center',
              justifyContent: 'space-between',
              gap: '16px',
              minHeight: '56px',
              paddingRight: '20px',
              paddingY: '12px',
            })}
          >
            <div class={css({ fontSize: '13px', fontWeight: 'medium', color: 'text.default', lineClamp: '1', wordBreak: 'break-all' })}>
              {publication.title ?? '(제목 없음)'}
            </div>

            <button
              class={center({
                flexShrink: '0',
                size: '24px',
                borderRadius: '4px',
                color: 'text.muted',
                _hover: { backgroundColor: 'surface.hover', color: 'danger.default' },
              })}
              disabled={busy}
              onclick={() => unpin(publication.id)}
              type="button"
              use:tooltip={{ message: '고정 해제', placement: 'top' }}
            >
              <Icon icon={PinOffIcon} size={14} />
            </button>
          </div>
        {/if}
      {/snippet}
    </ReorderableList>

    {#if pinned.length > 0}
      <SettingsDivider />
    {/if}

    <SettingsRow>
      {#snippet label()}
        {#if pinned.length === 0 && candidates.length === 0}
          아직 발행한 글이 없어요
        {:else}
          고정 추가
        {/if}
      {/snippet}
      {#snippet description()}
        {pinned.length} / {PIN_LIMIT}
      {/snippet}
      {#snippet value()}
        {#if canAdd}
          <Menu placement="bottom-end">
            {#snippet button()}
              <Button disabled={busy} size="sm" variant="secondary">
                <Icon icon={PlusIcon} size={14} />
                고정 추가
              </Button>
            {/snippet}

            {#each candidates as candidate (candidate.id)}
              <MenuItem onclick={() => pin(candidate.id)}>{candidate.title ?? '(제목 없음)'}</MenuItem>
            {/each}
          </Menu>
        {:else}
          <Button disabled size="sm" variant="secondary">
            <Icon icon={PlusIcon} size={14} />
            고정 추가
          </Button>
        {/if}
      {/snippet}
    </SettingsRow>
  </SettingsCard>
</div>
