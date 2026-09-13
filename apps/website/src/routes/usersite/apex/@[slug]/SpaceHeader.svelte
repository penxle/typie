<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon, Tooltip } from '@typie/ui/components';
  import ArrowUpRightIcon from '~icons/lucide/arrow-up-right';
  import SettingsIcon from '~icons/lucide/settings';
  import { Img } from '$lib/components';
  import { graphql } from '$mearie';
  import { currentSpaceSlug } from './current-space-slug';
  import { spaceHomePath } from './paths';
  import SpaceSettingsModal from './SpaceSettingsModal.svelte';
  import type { UsersiteSpace_SpaceHeader_spaceView$key } from '$mearie';

  type Props = {
    spaceView$key: UsersiteSpace_SpaceHeader_spaceView$key;
    variant: 'sidebar' | 'compact';
  };

  let { spaceView$key, variant }: Props = $props();

  let settingsOpen = $state(false);

  const space = createFragment(
    graphql(`
      fragment UsersiteSpace_SpaceHeader_spaceView on SpaceView {
        id
        name
        description
        availableActions
        publicationCount

        collections {
          id
        }

        links {
          label
          url
        }

        logo {
          id
          ...Img_image
        }
      }
    `),
    () => spaceView$key,
  );

  const slug = $derived(currentSpaceSlug());
  const sidebar = $derived(variant === 'sidebar');
</script>

<header class={flex({ flexDirection: 'column', gap: '12px' })}>
  <div class={flex({ alignItems: 'flex-start', justifyContent: 'space-between', gap: '12px' })}>
    <a class={css({ flexShrink: '0' })} aria-label={space.data.name} href={spaceHomePath(slug)}>
      <Img
        style={css.raw({
          size: sidebar ? '64px' : '48px',
          borderRadius: sidebar ? '16px' : '12px',
          objectFit: 'cover',
          boxShadow: '[inset 0 0 0 1px rgba(0, 0, 0, 0.06)]',
        })}
        alt={`${space.data.name} 로고`}
        image$key={space.data.logo}
        size={sidebar ? 128 : 96}
      />
    </a>

    {#if space.data.availableActions.includes('SETTINGS')}
      <Tooltip message="스페이스 설정">
        <button
          class={flex({
            flexShrink: '0',
            alignItems: 'center',
            gap: '6px',
            height: '32px',
            paddingX: '12px',
            borderWidth: '1px',
            borderColor: 'border.hairline',
            borderRadius: 'full',
            fontSize: '13px',
            fontWeight: 'medium',
            color: 'text.muted',
            backgroundColor: 'surface.default',
            transition: 'common',
            _hover: { backgroundColor: 'surface.hover', color: 'text.default' },
          })}
          onclick={() => {
            settingsOpen = true;
          }}
          type="button"
        >
          <Icon icon={SettingsIcon} size={14} />
          설정
        </button>
      </Tooltip>
    {/if}
  </div>

  <a href={spaceHomePath(slug)}>
    <h1 class={css({ fontSize: '20px', fontWeight: 'bold', letterSpacing: '-0.02em', lineHeight: '[1.25]', lineClamp: '2' })}>
      {space.data.name}
    </h1>
  </a>

  {#if space.data.description}
    <p class={css({ fontSize: '14px', lineHeight: '[1.6]', color: 'text.muted' })}>{space.data.description}</p>
  {/if}

  <div class={flex({ alignItems: 'center', gap: '6px', fontSize: '13px', color: 'text.hint', fontVariantNumeric: 'tabular-nums' })}>
    <span>글 {space.data.publicationCount}개</span>
    <i class={css({ flexShrink: '0', size: '2px', borderRadius: 'full', backgroundColor: 'border.emphasis' })} aria-hidden="true"></i>
    <span>시리즈 {space.data.collections.length}개</span>
  </div>

  {#if space.data.links.length > 0}
    <ul class={flex({ flexWrap: 'wrap', rowGap: '6px', columnGap: '14px' })}>
      {#each space.data.links as link, index (index)}
        <li>
          <a
            class={flex({
              display: 'inline-flex',
              alignItems: 'center',
              gap: '4px',
              fontSize: '13px',
              color: 'text.muted',
              transition: 'colors',
              _hover: { color: 'text.default' },
            })}
            href={link.url}
            rel="noopener noreferrer nofollow ugc"
            target="_blank"
          >
            {link.label}
            <Icon style={css.raw({ color: 'text.hint' })} icon={ArrowUpRightIcon} size={12} />
          </a>
        </li>
      {/each}
    </ul>
  {/if}
</header>

<SpaceSettingsModal
  onclose={() => {
    settingsOpen = false;
  }}
  open={settingsOpen}
  spaceId={space.data.id}
/>
