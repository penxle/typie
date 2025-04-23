<script lang="ts">
  import CheckIcon from '~icons/lucide/check';
  import CopyIcon from '~icons/lucide/copy';
  import ExternalLinkIcon from '~icons/lucide/external-link';
  import GlobeIcon from '~icons/lucide/globe';
  import LockIcon from '~icons/lucide/lock';
  import { fragment, graphql } from '$graphql';
  import { Button, Icon, Modal } from '$lib/components';
  import { css, cx } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';
  import type { DashboardLayout_ShareFolderModal_folder } from '$graphql';

  type Props = {
    open: boolean;
    $folder: DashboardLayout_ShareFolderModal_folder;
  };

  let { open = $bindable(), $folder: _folder }: Props = $props();

  const folder = fragment(
    _folder,
    graphql(`
      fragment DashboardLayout_ShareFolderModal_folder on Folder {
        id

        option {
          id
          visibility
        }

        entity {
          id
          url
        }
      }
    `),
  );

  const updateFolderOption = graphql(`
    mutation DashboardLayout_ShareFolderModal_UpdateFolderOption_Mutation($input: UpdateFolderOptionInput!) {
      updateFolderOption(input: $input) {
        id
        visibility
      }
    }
  `);

  let linkInputEl = $state<HTMLInputElement>();
  let copied = $state(false);
  let copiedTimeout = $state<NodeJS.Timeout>();

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

<Modal style={css.raw({ maxWidth: '440px' })} bind:open>
  <div class={flex({ flexDirection: 'column', gap: '16px' })}>
    <div class={flex({ flexDirection: 'column', gap: '12px' })}>
      <div class={flex({ justifyContent: 'space-between', alignItems: 'center', userSelect: 'none' })}>
        {#if $folder.option.visibility === 'PRIVATE'}
          <div class={center({ gap: '6px', borderRadius: 'full', paddingX: '10px', paddingY: '4px', backgroundColor: 'gray.100' })}>
            <div class={css({ size: '6px', borderRadius: 'full', bg: 'gray.500' })}></div>
            <div class={css({ fontSize: '12px', fontWeight: 'medium', color: 'gray.700' })}>비공개 중</div>
          </div>
        {:else}
          <div class={center({ gap: '6px', borderRadius: 'full', paddingX: '10px', paddingY: '4px', backgroundColor: 'brand.100' })}>
            <div class={css({ size: '6px', borderRadius: 'full', bg: 'brand.500' })}></div>
            <div class={css({ fontSize: '12px', fontWeight: 'medium', color: 'brand.700' })}>링크 공개 중</div>
          </div>
        {/if}

        {#if $folder.option.visibility !== 'PRIVATE'}
          <div class={center({ color: 'gray.500', fontSize: '12px' })}>링크가 있는 누구나 폴더 내의 링크 공유 포스트를 볼 수 있어요</div>
        {/if}
      </div>

      <div
        class={cx(
          'group',
          flex({
            alignItems: 'center',
            gap: '4px',
            borderWidth: '1px',
            borderRadius: '6px',
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
          value={$folder.entity.url}
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
          <Icon icon={copied ? CheckIcon : CopyIcon} size={14} />
        </button>

        <a
          class={center({
            borderRadius: '6px',
            size: '20px',
            color: 'gray.500',
            _hover: { color: 'gray.700', backgroundColor: 'gray.200' },
          })}
          href={$folder.entity.url}
          rel="noopener noreferrer"
          target="_blank"
        >
          <Icon icon={ExternalLinkIcon} size={14} />
        </a>
      </div>
    </div>

    {#if $folder.option.visibility === 'PRIVATE'}
      <Button
        style={css.raw({ height: '36px' })}
        onclick={async () => {
          await updateFolderOption({ folderId: $folder.id, visibility: 'UNLISTED' });
        }}
      >
        <div class={center({ gap: '6px' })}>
          <Icon icon={GlobeIcon} />
          <span>링크 공개로 전환</span>
        </div>
      </Button>
    {:else}
      <div class={flex({ justifyContent: 'flex-end' })}>
        <button
          class={center({ gap: '6px', color: 'gray.400', fontSize: '12px', _hover: { color: 'gray.500' } })}
          onclick={async () => {
            await updateFolderOption({ folderId: $folder.id, visibility: 'PRIVATE' });
          }}
          type="button"
        >
          <Icon icon={LockIcon} size={12} />
          비공개로 전환
        </button>
      </div>
    {/if}
  </div>
</Modal>
