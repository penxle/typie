<script lang="ts">
  import { createFragment, createMutation } from '@mearie/svelte';
  import { EntityVisibility } from '@typie/lib/enums';
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { Button, Icon, RingSpinner } from '@typie/ui/components';
  import { Toast } from '@typie/ui/notification';
  import mixpanel from 'mixpanel-browser';
  import CheckIcon from '~icons/lucide/check';
  import CopyIcon from '~icons/lucide/copy';
  import ExternalLinkIcon from '~icons/lucide/external-link';
  import Layers2Icon from '~icons/lucide/layers-2';
  import LinkIcon from '~icons/lucide/link';
  import LockIcon from '~icons/lucide/lock';
  import { publicationErrorCode } from '$lib/publication/error';
  import { publicationErrorMessage, sharedValue } from '$lib/publication/publish-form';
  import { graphql } from '$mearie';
  import { SubscribeModal } from '../@subscription/subscribe-modal.svelte';
  import OptionCard from './OptionCard.svelte';
  import { groupLabelStyle, linkFieldButtonStyle, linkFieldInputStyle, linkFieldStyle } from './publish-styles';
  import ShareHeader from './ShareHeader.svelte';
  import type { DashboardLayout_Share_Folder_folder$key } from '$mearie';

  type Props = {
    folders$key: DashboardLayout_Share_Folder_folder$key[];
    onclose: () => void;
  };

  let { folders$key, onclose }: Props = $props();

  const folders = createFragment(
    graphql(`
      fragment DashboardLayout_Share_Folder_folder on Folder {
        id
        name

        entity {
          id
          visibility
          url
        }
      }
    `),
    () => folders$key,
  );

  const [updateFoldersOption] = createMutation(
    graphql(`
      mutation DashboardLayout_Share_Folder_UpdateFoldersOption_Mutation($input: UpdateFoldersOptionInput!) {
        updateFoldersOption(input: $input) {
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
    `),
  );

  const multiple = $derived(folders.data.length > 1);
  const folderIds = $derived(folders.data.map((folder) => folder.id));
  const visibility = $derived(sharedValue(folders.data.map((folder) => folder.entity.visibility)));
  const mixed = $derived(multiple && visibility === undefined);

  const countOf = (value: EntityVisibility) => folders.data.filter((folder) => folder.entity.visibility === value).length;
  const hint = (value: EntityVisibility) => (mixed ? `${countOf(value)}개` : null);

  let copied = $state(false);
  let timer: ReturnType<typeof setTimeout> | undefined;
  let recursiveState = $state<'idle' | 'inflight' | 'success'>('idle');
  let recursiveTimer: ReturnType<typeof setTimeout> | undefined;

  $effect(() => {
    return () => {
      if (timer) clearTimeout(timer);
      if (recursiveTimer) clearTimeout(recursiveTimer);
    };
  });

  const selectUrl = (event: Event) => {
    (event.currentTarget as HTMLInputElement).select();
  };

  const copyLinks = async () => {
    await navigator.clipboard.writeText(folders.data.map((folder) => folder.entity.url).join('\n'));
    mixpanel.track('copy_folder_share_url', { count: folders.data.length });

    if (timer) clearTimeout(timer);
    copied = true;
    timer = setTimeout(() => (copied = false), 2000);
  };

  const setVisibility = async (next: EntityVisibility) => {
    if (visibility === next) return;
    if (!SubscribeModal.gate('share_folder')) return;

    try {
      await updateFoldersOption({ input: { folderIds, visibility: next } });
      mixpanel.track('update_folder_option', { visibility: next, count: folderIds.length });
    } catch (err) {
      Toast.error(publicationErrorMessage(publicationErrorCode(err)));
    }
  };

  const applyRecursive = async () => {
    if (recursiveState === 'inflight' || visibility === undefined) return;
    if (recursiveTimer) clearTimeout(recursiveTimer);
    if (!SubscribeModal.gate('share_folder')) return;

    recursiveState = 'inflight';

    try {
      await updateFoldersOption({ input: { folderIds, visibility, recursive: true } });
      recursiveState = 'success';
      mixpanel.track('update_folder_option', { visibility, recursive: true });
    } catch (err) {
      recursiveState = 'idle';
      Toast.error(publicationErrorMessage(publicationErrorCode(err)));
      return;
    }

    recursiveTimer = setTimeout(() => {
      recursiveState = 'idle';
    }, 2000);
  };
</script>

{#snippet linkBody()}
  {#if multiple}
    <Button style={css.raw({ alignSelf: 'flex-start', gap: '6px' })} onclick={copyLinks} size="sm" variant="secondary">
      <Icon style={copied ? css.raw({ color: 'success.default' }) : undefined} icon={copied ? CheckIcon : CopyIcon} size={14} />
      {copied ? '복사되었어요' : `링크 ${folders.data.length}개 복사`}
    </Button>
  {:else}
    <div class={css(linkFieldStyle)}>
      <input
        class={css(linkFieldInputStyle)}
        aria-label="공유 링크"
        autocomplete="off"
        onclick={selectUrl}
        onfocus={selectUrl}
        readonly
        spellcheck="false"
        value={folders.data[0].entity.url}
      />

      <button
        class={center(linkFieldButtonStyle)}
        aria-label="링크 복사"
        onclick={copyLinks}
        type="button"
        use:tooltip={{ message: copied ? '복사되었어요' : '링크 복사', placement: 'top', keepOnClick: true }}
      >
        <Icon style={copied ? css.raw({ color: 'success.default' }) : undefined} icon={copied ? CheckIcon : CopyIcon} size={14} />
      </button>

      <a
        class={center(linkFieldButtonStyle)}
        aria-label="조회 페이지에서 열기"
        href={folders.data[0].entity.url}
        rel="noopener noreferrer"
        target="_blank"
        use:tooltip={{ message: '조회 페이지에서 열기', placement: 'top' }}
      >
        <Icon icon={ExternalLinkIcon} size={14} />
      </a>
    </div>
  {/if}
{/snippet}

<ShareHeader {onclose} subtitle={multiple ? `폴더 ${folders.data.length}개` : folders.data[0].name} title="공유" />

<div class={flex({ flexDirection: 'column', gap: '24px', paddingTop: '2px', paddingX: '24px', paddingBottom: '24px' })}>
  <section class={flex({ flexDirection: 'column', gap: '10px' })}>
    <div class={css(groupLabelStyle)}>공개 범위</div>

    <div class={flex({ flexDirection: 'column', gap: '8px' })} aria-label="공개 범위" role="radiogroup">
      <OptionCard
        description="나만 볼 수 있어요."
        hint={hint(EntityVisibility.PRIVATE)}
        icon={LockIcon}
        label="비공개"
        onclick={() => setVisibility(EntityVisibility.PRIVATE)}
        selected={visibility === EntityVisibility.PRIVATE}
      />
      <OptionCard
        body={linkBody}
        description="링크가 있는 누구나 폴더와 폴더 내의 링크 공개 문서를 볼 수 있어요."
        hint={hint(EntityVisibility.UNLISTED)}
        icon={LinkIcon}
        label="링크가 있는 사람"
        onclick={() => setVisibility(EntityVisibility.UNLISTED)}
        selected={visibility === EntityVisibility.UNLISTED}
      />
    </div>

    {#if mixed}
      <p class={css({ fontSize: '12px', lineHeight: '[1.5]', color: 'text.hint' })}>
        공개 범위가 서로 달라요. 고르면 폴더 {folders.data.length}개에 모두 적용돼요.
      </p>
    {/if}

    <div class={flex({ alignItems: 'center', justifyContent: 'flex-end', gap: '8px' })}>
      <span class={css({ fontSize: '12px', color: 'text.hint' })}>발행된 글은 그대로 두고 나머지에 적용돼요.</span>

      <Button
        style={css.raw({ minWidth: '200px', gap: '4px' })}
        disabled={visibility === undefined}
        onclick={applyRecursive}
        size="sm"
        variant="secondary"
      >
        {#if recursiveState === 'inflight'}
          <RingSpinner style={css.raw({ size: '14px' })} />
          적용 중...
        {:else if recursiveState === 'success'}
          <Icon icon={CheckIcon} size={14} />
          적용됨
        {:else}
          <Icon icon={Layers2Icon} size={14} />
          하위 항목에 동일한 설정 적용하기
        {/if}
      </Button>
    </div>
  </section>
</div>
