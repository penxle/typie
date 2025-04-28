<script lang="ts">
  import { fade } from 'svelte/transition';
  import CheckIcon from '~icons/lucide/check';
  import CopyIcon from '~icons/lucide/copy';
  import ShareIcon from '~icons/lucide/share';
  import XIcon from '~icons/lucide/x';
  import MastodonIcon from '~icons/simple-icons/mastodon';
  import TwitterIcon from '~icons/simple-icons/twitter';
  import { createFloatingActions, portal } from '$lib/actions';
  import { Icon } from '$lib/components';
  import { css, cx } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';

  type Props = {
    href: string;
  };

  let { href }: Props = $props();

  let open = $state(false);

  let linkInputEl = $state<HTMLInputElement>();
  let copied = $state(false);
  let copiedTimeout = $state<NodeJS.Timeout>();

  const { anchor, floating } = createFloatingActions({
    placement: 'bottom',
    offset: 8,
    onClickOutside: () => {
      open = false;
    },
  });

  const handleCopyLink = () => {
    if (!linkInputEl) {
      return;
    }

    navigator.clipboard.writeText(linkInputEl.value);

    if (copiedTimeout) {
      clearTimeout(copiedTimeout);
    }

    copied = true;
    copiedTimeout = setTimeout(() => (copied = false), 2000);
  };
</script>

<button
  class={css({ marginLeft: 'auto', borderRadius: '4px', padding: '3px', _hover: { backgroundColor: 'gray.100' } })}
  onclick={() => (open = true)}
  type="button"
  use:anchor
>
  <Icon icon={ShareIcon} size={16} />
</button>

{#if open}
  <div
    class={css({
      position: 'fixed',
      inset: '0',
      zIndex: '1',
      lgDown: {
        backgroundColor: 'gray.900/30',
        transition: 'opacity',
        zIndex: '50',
      },
    })}
    onclick={() => (open = false)}
    onkeypress={null}
    role="button"
    tabindex="-1"
    use:portal
    transition:fade={{ duration: 100 }}
  ></div>

  <div
    class={css({
      display: 'flex',
      flexDirection: 'column',
      gap: '12px',
      borderColor: 'gray.200',
      paddingX: '18px',
      backgroundColor: 'white',
      width: 'full',
      boxShadow: 'small',
      zIndex: '50',
      lgDown: {
        position: '[fixed!]',
        top: '[initial!]',
        bottom: '[0!]',
        left: '[0!]',
        right: '[0!]',
        borderTopWidth: '1px',
        borderTopRadius: '8px',
      },
      lg: { borderWidth: '1px', borderRadius: '8px', maxWidth: '360px' },
    })}
    use:floating
    transition:fade={{ duration: 100 }}
  >
    <div class={css({ position: 'relative', marginY: '14px', fontWeight: 'medium', textAlign: 'center' })}>
      공유하기

      <button
        class={css({
          position: 'absolute',
          top: '1/2',
          right: '0',
          translate: 'auto',
          translateY: '-1/2',
          borderRadius: '4px',
          padding: '2px',
          _hover: { backgroundColor: 'gray.200' },
        })}
        onclick={() => (open = false)}
        type="button"
      >
        <Icon icon={XIcon} size={16} />
      </button>
    </div>

    <div class={flex({ gap: '32px', marginX: 'auto' })}>
      <div>
        <a
          class={center({ borderRadius: '12px', backgroundColor: 'gray.100', size: '60px', _hover: { backgroundColor: 'gray.200' } })}
          href={`https://x.com/intent/post?text=${href}`}
          rel="noopener noreferrer"
          target="_blank"
        >
          <Icon style={css.raw({ color: '[#1D9BF0]' })} icon={TwitterIcon} size={24} />
        </a>
        <p class={css({ marginTop: '6px', fontSize: '14px', textAlign: 'center', color: 'gray.700' })}>트위터</p>
      </div>

      <div>
        <a
          class={center({ borderRadius: '12px', backgroundColor: 'gray.100', size: '60px', _hover: { backgroundColor: 'gray.200' } })}
          href={`https://share.planet.moe/share?text=${href}`}
          rel="noopener noreferrer"
          target="_blank"
        >
          <Icon style={css.raw({ color: '[#6364FF]' })} icon={MastodonIcon} size={24} />
        </a>
        <p class={css({ marginTop: '6px', fontSize: '14px', textAlign: 'center', color: 'gray.700' })}>마스토돈</p>
      </div>
    </div>

    <div
      class={cx(
        'group',
        flex({
          alignItems: 'center',
          gap: '4px',
          borderWidth: '1px',
          borderRadius: '6px',
          marginY: '16px',
          paddingX: '12px',
          height: '36px',
          backgroundColor: 'gray.50',
          _hover: {
            borderColor: 'brand.200',
          },
        }),
      )}
    >
      <input
        bind:this={linkInputEl}
        class={css({ flexGrow: '1', color: 'gray.600', fontSize: '12px', _groupHover: { color: 'gray.900' } })}
        onclick={() => linkInputEl?.select()}
        readonly
        value={href}
      />

      <button
        class={center({
          borderRadius: '6px',
          size: '20px',
          color: 'gray.500',
          _hover: { color: 'gray.700', backgroundColor: 'gray.200' },
        })}
        onclick={handleCopyLink}
        type="button"
      >
        <Icon data-floating-keep-open icon={copied ? CheckIcon : CopyIcon} size={14} />
      </button>
    </div>
  </div>
{/if}
