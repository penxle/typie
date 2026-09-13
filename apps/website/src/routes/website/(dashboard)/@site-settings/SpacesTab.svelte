<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { Icon } from '@typie/ui/components';
  import ArrowUpRightIcon from '~icons/lucide/arrow-up-right';
  import ExternalLinkIcon from '~icons/lucide/external-link';
  import PlusIcon from '~icons/lucide/plus';
  import SettingsIcon from '~icons/lucide/settings';
  import { env } from '$env/dynamic/public';
  import { Img, SettingsCard, SettingsDivider, SettingsRow } from '$lib/components';
  import { graphql } from '$mearie';
  import CreateSpaceModal from './CreateSpaceModal.svelte';
  import type { DashboardLayout_SiteSettingsModal_SpacesTab_site$key } from '$mearie';

  type Props = {
    site$key: DashboardLayout_SiteSettingsModal_SpacesTab_site$key;
  };

  let { site$key }: Props = $props();

  let createOpen = $state(false);

  const site = createFragment(
    graphql(`
      fragment DashboardLayout_SiteSettingsModal_SpacesTab_site on Site {
        id

        logo {
          id
          ...Img_image
        }

        spaces {
          id
          name
          slug
          url

          logo {
            id
            ...Img_image
          }
        }
      }
    `),
    () => site$key,
  );
</script>

<div class={css({ maxWidth: '640px' })}>
  <div class={flex({ alignItems: 'flex-start', justifyContent: 'space-between', gap: '16px', marginBottom: '24px' })}>
    <div>
      <h1 class={css({ fontSize: '20px', fontWeight: 'semibold', color: 'text.default', marginBottom: '4px' })}>스페이스</h1>
      <p class={css({ fontSize: '13px', color: 'text.muted', lineHeight: '[1.6]' })}>
        스페이스는 발행한 글이 모이는 공개 공간이에요. 작업실의 문서를 스페이스에 발행하면 누구나 읽을 수 있어요.
      </p>
    </div>

    <button
      class={flex({
        flexShrink: '0',
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
      })}
      onclick={() => {
        createOpen = true;
      }}
      type="button"
    >
      <Icon style={css.raw({ color: 'text.muted' })} icon={PlusIcon} size={14} />
      <span>새 스페이스</span>
    </button>
  </div>

  {#if site.data.spaces.length > 0}
    <SettingsCard>
      {#each site.data.spaces as space, index (space.id)}
        {#if index > 0}
          <SettingsDivider />
        {/if}

        <SettingsRow>
          {#snippet label()}
            <div class={flex({ alignItems: 'center', gap: '12px' })}>
              <Img
                style={css.raw({ size: '32px', borderRadius: '4px', flexShrink: '0' })}
                alt={space.name}
                image$key={space.logo ?? site.data.logo}
                size={64}
              />
              <span class={css({ lineClamp: '1', wordBreak: 'break-all' })}>{space.name}</span>
              <a
                class={flex({
                  flexShrink: '0',
                  alignItems: 'center',
                  gap: '2px',
                  borderRadius: '4px',
                  paddingX: '8px',
                  paddingY: '2px',
                  fontSize: '12px',
                  fontWeight: 'medium',
                  color: 'text.muted',
                  backgroundColor: 'surface.inset',
                  transition: 'common',
                  _hover: { color: 'text.default', backgroundColor: 'surface.hover' },
                })}
                href={space.url}
                rel="noopener noreferrer"
                target="_blank"
              >
                {env.PUBLIC_USERSITE_HOST}/@{space.slug}
                <Icon icon={ArrowUpRightIcon} size={12} />
              </a>
            </div>
          {/snippet}
          {#snippet value()}
            <div class={flex({ alignItems: 'center', gap: '4px' })}>
              <button
                class={center({
                  size: '24px',
                  borderRadius: '4px',
                  color: 'text.muted',
                  _hover: { backgroundColor: 'surface.hover', color: 'text.default' },
                })}
                type="button"
                use:tooltip={{ message: '설정', placement: 'top' }}
              >
                <Icon icon={SettingsIcon} size={14} />
              </button>
              <a
                class={center({
                  size: '24px',
                  borderRadius: '4px',
                  color: 'text.muted',
                  _hover: { backgroundColor: 'surface.hover', color: 'text.default' },
                })}
                href={space.url}
                rel="noopener noreferrer"
                target="_blank"
                use:tooltip={{ message: '스페이스 열기', placement: 'top' }}
              >
                <Icon icon={ExternalLinkIcon} size={14} />
              </a>
            </div>
          {/snippet}
        </SettingsRow>
      {/each}
    </SettingsCard>
  {:else}
    <SettingsCard>
      <div class={css({ paddingX: '20px', paddingY: '40px', fontSize: '13px', color: 'text.muted', textAlign: 'center' })}>
        아직 만든 스페이스가 없어요.
        <br />
        우측 상단의 새 스페이스 버튼으로 첫 스페이스를 만들 수 있어요.
      </div>
    </SettingsCard>
  {/if}
</div>

<CreateSpaceModal siteId={site.data.id} bind:open={createOpen} />
