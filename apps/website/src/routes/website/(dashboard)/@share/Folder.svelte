<script lang="ts">
  import { createFragment, createMutation } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { Button, HorizontalDivider, Icon, RingSpinner, Select } from '@typie/ui/components';
  import { createForm } from '@typie/ui/form';
  import mixpanel from 'mixpanel-browser';
  import { z } from 'zod';
  import { EntityVisibility } from '@/enums';
  import BlendIcon from '~icons/lucide/blend';
  import CheckIcon from '~icons/lucide/check';
  import GlobeIcon from '~icons/lucide/globe';
  import ImageIcon from '~icons/lucide/image';
  import Layers2Icon from '~icons/lucide/layers-2';
  import LinkIcon from '~icons/lucide/link';
  import LockIcon from '~icons/lucide/lock';
  import Trash2Icon from '~icons/lucide/trash-2';
  import { Img } from '$lib/components';
  import { uploadBlobAsImage } from '$lib/utils';
  import { graphql } from '$mearie';
  import type { DashboardLayout_Share_Folder_folder$key } from '$mearie';

  type Props = {
    folders$key: DashboardLayout_Share_Folder_folder$key[];
  };

  let { folders$key }: Props = $props();

  const folders = createFragment(
    graphql(`
      fragment DashboardLayout_Share_Folder_folder on Folder {
        id
        name

        thumbnail {
          id
          ...Img_image
        }

        entity {
          id
          visibility
          url
        }
      }
    `),
    () => folders$key,
  );

  const isSingleFolder = $derived(folders.data.length === 1);
  const folderIds = $derived(folders.data.map((f) => f.id));

  const [updateFoldersOption] = createMutation(
    graphql(`
      mutation DashboardLayout_Share_Folder_UpdateFoldersOption_Mutation($input: UpdateFoldersOptionInput!) {
        updateFoldersOption(input: $input) {
          id

          thumbnail {
            id
            ...Img_image
          }

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
    `),
  );

  let copied = $state(false);
  let timer: NodeJS.Timeout | undefined;

  let recursiveState = $state<'idle' | 'inflight' | 'success'>('idle');
  let recursiveTimer: NodeJS.Timeout | undefined;
  let thumbnailUploading = $state(false);

  const form = createForm({
    schema: z.object({
      visibility: z.nativeEnum(EntityVisibility),
    }),
    submitOn: 'change',
    onSubmit: async (data) => {
      if (folders.data.length === 0) return;

      const dirtyFields = form.getDirtyFields();
      const updateData: {
        folderIds: string[];
        visibility?: EntityVisibility;
      } = { folderIds };

      if ('visibility' in dirtyFields) {
        updateData.visibility = data.visibility;
      }

      if (Object.keys(updateData).length > 1) {
        await updateFoldersOption({ input: updateData });
        mixpanel.track('update_folder_option', { visibility: data.visibility, count: folderIds.length });
      }
    },
    defaultValues: {
      visibility: folders.data[0].entity.visibility,
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

  const visibilityIndeterminate = $derived(
    folders.data.length > 1 && folders.data.some((f) => f.entity.visibility !== folders.data[0].entity.visibility),
  );
  const thumbnailIndeterminate = $derived(
    folders.data.length > 1 && folders.data.some((f) => f.thumbnail?.id !== folders.data[0].thumbnail?.id),
  );

  const handleCopyLink = () => {
    if (folders.data.length === 0) return;

    const urls = folders.data.map((f) => f.entity.url).join('\n');
    navigator.clipboard.writeText(urls);
    mixpanel.track('copy_folder_share_url', { count: folders.data.length });

    if (timer) {
      clearTimeout(timer);
    }

    copied = true;
    timer = setTimeout(() => (copied = false), 2000);
  };

  const handleThumbnailUpload = () => {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = 'image/*';

    input.addEventListener('change', async () => {
      const file = input.files?.[0];
      if (!file) return;

      thumbnailUploading = true;
      try {
        const image = await uploadBlobAsImage(file);
        await updateFoldersOption({ input: { folderIds, thumbnailId: image.id } });
        mixpanel.track('update_folder_thumbnail', { count: folders.data.length });
      } finally {
        thumbnailUploading = false;
      }
    });

    input.click();
  };

  const handleThumbnailRemove = async () => {
    await updateFoldersOption({ input: { folderIds, thumbnailId: null } });
    mixpanel.track('remove_folder_thumbnail', { count: folders.data.length });
  };
</script>

<div class={flex({ justifyContent: 'space-between', alignItems: 'center', gap: '32px', paddingX: '16px', paddingY: '12px' })}>
  <div class={flex({ gap: '[0.5ch]', fontSize: '12px', fontWeight: 'medium' })}>
    <span class={css({ wordBreak: 'break-all', lineClamp: '1', fontWeight: 'semibold' })}>
      {isSingleFolder ? folders.data[0].name : `${folders.data.length}개의 폴더`}
    </span>
    <span class={css({ flexShrink: '0' })}>공유 및 게시하기</span>
  </div>

  <button
    class={flex({ alignItems: 'center', gap: '4px', flexShrink: '0' })}
    onclick={handleCopyLink}
    type="button"
    use:tooltip={{
      message: visibilityIndeterminate
        ? null
        : form.fields.visibility === EntityVisibility.PRIVATE
          ? '지금은 링크가 있어도 나만 볼 수 있어요'
          : '링크가 있는 누구나 폴더와 폴더 내의 링크 공개 문서를 볼 수 있어요',
      placement: 'top',
      keepOnClick: true,
    }}
  >
    {#if copied}
      <Icon style={css.raw({ color: 'text.link' })} icon={CheckIcon} size={12} />
      <div class={css({ fontSize: '12px', color: 'text.link' })}>복사되었어요</div>
    {:else}
      <Icon style={css.raw({ color: 'text.link' })} icon={LinkIcon} size={12} />
      <div class={css({ fontSize: '12px', color: 'text.link' })}>
        {isSingleFolder ? '게시 링크 복사' : '게시 링크 모두 복사'}
      </div>
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
            icon: GlobeIcon,
            label: '공개',
            description: '누구나 폴더와 폴더 내의 공개 문서를 볼 수 있어요.',
            value: EntityVisibility.PUBLIC,
          },
          {
            icon: LinkIcon,
            label: '링크가 있는 사람',
            description: '링크가 있는 누구나 폴더와 폴더 내의 링크 공개 문서를 볼 수 있어요.',
            value: EntityVisibility.UNLISTED,
          },
          {
            icon: LockIcon,
            label: '비공개',
            description: '나만 볼 수 있어요.',
            value: EntityVisibility.PRIVATE,
          },
        ]}
        values={folders.data.map((f) => f.entity.visibility)}
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

        await updateFoldersOption({ input: { folderIds, visibility: form.fields.visibility, recursive: true } });

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

  <div class={flex({ flexDirection: 'column', gap: '12px' })}>
    <div class={css({ fontSize: '12px', fontWeight: 'medium', color: 'text.faint' })}>썸네일</div>

    <div class={flex({ alignItems: 'center', justifyContent: 'space-between' })}>
      <div class={flex({ alignItems: 'center', gap: '8px' })}>
        <Icon style={css.raw({ color: 'text.faint' })} icon={ImageIcon} />
        <div class={css({ fontSize: '12px', color: 'text.subtle' })}>미리보기 이미지</div>
      </div>

      <div class={flex({ gap: '8px', alignItems: 'center' })}>
        {#if thumbnailIndeterminate}
          <button
            class={center({
              width: '64px',
              height: '36px',
              borderRadius: '6px',
              backgroundColor: 'surface.muted',
              fontSize: '10px',
              color: 'text.faint',
            })}
            disabled={thumbnailUploading}
            onclick={handleThumbnailUpload}
            type="button"
          >
            {thumbnailUploading ? '...' : '다름'}
          </button>
        {:else if folders.data[0].thumbnail}
          <div class={flex({ alignItems: 'center', gap: '4px' })}>
            <button
              class={css({ position: 'relative', cursor: 'pointer' })}
              disabled={thumbnailUploading}
              onclick={handleThumbnailUpload}
              type="button"
            >
              <Img
                style={css.raw({
                  width: '64px',
                  height: '36px',
                  borderRadius: '6px',
                  objectFit: 'cover',
                })}
                alt="썸네일"
                image$key={folders.data[0].thumbnail}
                size={128}
              />
            </button>
            <button
              class={center({
                size: '24px',
                borderRadius: '4px',
                color: 'text.faint',
                _hover: { backgroundColor: 'surface.muted', color: 'text.danger' },
              })}
              onclick={handleThumbnailRemove}
              type="button"
              use:tooltip={{ message: '삭제', placement: 'top' }}
            >
              <Icon icon={Trash2Icon} size={14} />
            </button>
          </div>
        {:else}
          <button
            class={center({
              width: '64px',
              height: '36px',
              borderWidth: '1px',
              borderStyle: 'dashed',
              borderRadius: '6px',
              color: 'text.faint',
              _hover: { backgroundColor: 'surface.muted' },
            })}
            disabled={thumbnailUploading}
            onclick={handleThumbnailUpload}
            type="button"
          >
            {#if thumbnailUploading}
              <span class={css({ fontSize: '10px' })}>...</span>
            {:else}
              <Icon icon={ImageIcon} size={14} />
            {/if}
          </button>
        {/if}
      </div>
    </div>
  </div>
</div>
