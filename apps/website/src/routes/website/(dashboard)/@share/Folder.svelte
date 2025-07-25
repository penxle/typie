<script lang="ts">
  import mixpanel from 'mixpanel-browser';
  import { z } from 'zod';
  import { EntityVisibility } from '@/enums';
  import BlendIcon from '~icons/lucide/blend';
  import CheckIcon from '~icons/lucide/check';
  import Layers2Icon from '~icons/lucide/layers-2';
  import LinkIcon from '~icons/lucide/link';
  import LockIcon from '~icons/lucide/lock';
  import { fragment, graphql } from '$graphql';
  import { tooltip } from '$lib/actions';
  import { Button, HorizontalDivider, Icon, RingSpinner, Select } from '$lib/components';
  import { createForm } from '$lib/form';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
  import type { DashboardLayout_Share_Folder_folder } from '$graphql';

  type Props = {
    $folder: DashboardLayout_Share_Folder_folder;
  };

  let { $folder: _folder }: Props = $props();

  const folder = fragment(
    _folder,
    graphql(`
      fragment DashboardLayout_Share_Folder_folder on Folder {
        id
        name

        entity {
          id
          url
          visibility
        }
      }
    `),
  );

  const updateFolderOption = graphql(`
    mutation DashboardLayout_Share_Folder_UpdateFolderOption_Mutation($input: UpdateFolderOptionInput!) {
      updateFolderOption(input: $input) {
        id

        entity {
          id
          visibility

          children {
            id
            visibility

            children {
              id
              visibility

              children {
                id
                visibility
              }
            }
          }
        }
      }
    }
  `);

  let copied = $state(false);
  let timer: NodeJS.Timeout | undefined;

  let recursiveState = $state<'idle' | 'inflight' | 'success'>('idle');
  let recursiveTimer: NodeJS.Timeout | undefined;

  const form = createForm({
    schema: z.object({
      visibility: z.nativeEnum(EntityVisibility),
    }),
    submitOn: 'change',
    onSubmit: async (data) => {
      await updateFolderOption({
        folderId: $folder.id,
        visibility: data.visibility,
      });

      mixpanel.track('update_folder_option', { visibility: data.visibility });
    },
    defaultValues: {
      visibility: $folder.entity.visibility,
    },
  });

  $effect(() => {
    void form;
  });

  $effect(() => {
    return () => {
      if (recursiveTimer) {
        clearTimeout(recursiveTimer);
      }
    };
  });

  const handleCopyLink = () => {
    navigator.clipboard.writeText($folder.entity.url);
    mixpanel.track('copy_folder_share_url');

    if (timer) {
      clearTimeout(timer);
    }

    copied = true;
    timer = setTimeout(() => (copied = false), 2000);
  };
</script>

<div class={flex({ justifyContent: 'space-between', alignItems: 'center', gap: '32px', paddingX: '16px', paddingY: '12px' })}>
  <div class={flex({ gap: '[0.5ch]', fontSize: '12px', fontWeight: 'medium' })}>
    <span class={css({ wordBreak: 'break-all', lineClamp: '1' })}>{$folder.name}</span>
    <span class={css({ flexShrink: '0' })}>공유 및 게시하기</span>
  </div>

  <button
    class={flex({ alignItems: 'center', gap: '4px', flexShrink: '0' })}
    onclick={handleCopyLink}
    type="button"
    use:tooltip={{
      message:
        form.fields.visibility === EntityVisibility.PRIVATE
          ? '지금은 링크가 있어도 나만 볼 수 있어요'
          : '링크가 있는 누구나 폴더와 폴더 내의 링크 공개 포스트를 볼 수 있어요',
      placement: 'top',
      keepOnClick: true,
    }}
  >
    {#if copied}
      <Icon style={css.raw({ color: 'text.link' })} icon={CheckIcon} size={12} />
      <div class={css({ fontSize: '12px', color: 'text.link' })}>복사되었어요</div>
    {:else}
      <Icon style={css.raw({ color: 'text.link' })} icon={LinkIcon} size={12} />
      <div class={css({ fontSize: '12px', color: 'text.link' })}>게시 링크 복사</div>
    {/if}
  </button>
</div>

<HorizontalDivider />

<div class={flex({ flexDirection: 'column', gap: '16px', paddingX: '16px', paddingTop: '16px', paddingBottom: '24px' })}>
  <div class={flex({ flexDirection: 'column', gap: '12px' })}>
    <div class={css({ fontSize: '12px', fontWeight: 'medium', color: 'text.faint' })}>폴더 조회 권한</div>

    <div class={flex({ alignItems: 'center', justifyContent: 'space-between', height: '24px' })}>
      <div class={flex({ alignItems: 'center', gap: '8px' })}>
        <Icon style={css.raw({ color: 'text.faint' })} icon={BlendIcon} size={14} />
        <div class={css({ fontSize: '12px', color: 'text.subtle' })}>공개 범위</div>
      </div>

      <Select
        items={[
          {
            icon: LinkIcon,
            label: '링크가 있는 사람',
            description: '링크가 있는 누구나 폴더와 폴더 내의 링크 공개 포스트를 볼 수 있어요.',
            value: EntityVisibility.UNLISTED,
          },
          {
            icon: LockIcon,
            label: '비공개',
            description: '나만 볼 수 있어요.',
            value: EntityVisibility.PRIVATE,
          },
        ]}
        bind:value={form.fields.visibility}
      />
    </div>

    <HorizontalDivider />

    <Button
      style={css.raw({ marginLeft: 'auto', minWidth: '200px', height: '26px', gap: '4px', fontSize: '12px' })}
      onclick={async () => {
        if (recursiveState === 'inflight') {
          return;
        }

        if (recursiveTimer) {
          clearTimeout(recursiveTimer);
        }

        recursiveState = 'inflight';

        await updateFolderOption({ folderId: $folder.id, visibility: form.fields.visibility, recursive: true });

        recursiveState = 'success';
        mixpanel.track('update_folder_option', { visibility: form.fields.visibility, recursive: true });

        recursiveTimer = setTimeout(() => {
          recursiveState = 'idle';
        }, 2000);
      }}
      size="sm"
      variant="secondary"
    >
      {#if recursiveState === 'inflight'}
        <RingSpinner style={css.raw({ size: '14px' })} />
        적용중...
      {:else if recursiveState === 'success'}
        <Icon icon={CheckIcon} size={14} />
        적용됨
      {:else}
        <Icon icon={Layers2Icon} size={14} />
        하위 요소에 동일한 설정 적용하기
      {/if}
    </Button>
  </div>
</div>
