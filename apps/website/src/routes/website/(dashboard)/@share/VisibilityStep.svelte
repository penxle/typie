<script lang="ts">
  import { createFragment, createMutation } from '@mearie/svelte';
  import { EntityVisibility } from '@typie/lib/enums';
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { Button, Icon } from '@typie/ui/components';
  import { Toast } from '@typie/ui/notification';
  import mixpanel from 'mixpanel-browser';
  import CheckIcon from '~icons/lucide/check';
  import CopyIcon from '~icons/lucide/copy';
  import ExternalLinkIcon from '~icons/lucide/external-link';
  import LinkIcon from '~icons/lucide/link';
  import LockIcon from '~icons/lucide/lock';
  import SendIcon from '~icons/lucide/send';
  import { publicationErrorCode } from '$lib/publication/error';
  import { publicationErrorMessage, sharedValue } from '$lib/publication/publish-form';
  import { graphql } from '$mearie';
  import { SubscribeModal } from '../@subscription/subscribe-modal.svelte';
  import OptionCard from './OptionCard.svelte';
  import { groupLabelStyle, linkFieldButtonStyle, linkFieldInputStyle, linkFieldStyle } from './publish-styles';
  import ReadingSettings from './ReadingSettings.svelte';
  import type { DashboardLayout_Share_VisibilityStep_document$key } from '$mearie';

  type Props = {
    documents$key: DashboardLayout_Share_VisibilityStep_document$key[];
    onPublishStep: () => void;
  };

  let { documents$key, onPublishStep }: Props = $props();

  const documents = createFragment(
    graphql(`
      fragment DashboardLayout_Share_VisibilityStep_document on Document {
        id

        entity {
          id
          url
          visibility
        }

        publication {
          id
          state
        }

        ...DashboardLayout_Share_ReadingSettings_document
      }
    `),
    () => documents$key,
  );

  const [updateDocumentsOption] = createMutation(
    graphql(`
      mutation DashboardLayout_Share_VisibilityStep_UpdateDocumentsOption_Mutation($input: UpdateDocumentsOptionInput!) {
        updateDocumentsOption(input: $input) {
          id

          entity {
            id
            visibility
          }
        }
      }
    `),
  );

  const multiple = $derived(documents.data.length > 1);
  const publishedCount = $derived(documents.data.filter((document) => document.publication?.state === 'PUBLISHED').length);
  const changeable = $derived(documents.data.filter((document) => document.publication?.state !== 'PUBLISHED'));
  const changeableIds = $derived(changeable.map((document) => document.id));
  const allPublished = $derived(changeableIds.length === 0);
  const anyPublishing = $derived(documents.data.some((document) => document.publication && document.publication.state !== 'UNPUBLISHED'));
  const visibility = $derived(sharedValue(changeable.map((document) => document.entity.visibility)));
  const mixed = $derived(multiple && changeable.length > 1 && visibility === undefined);

  const countOf = (value: EntityVisibility) => changeable.filter((document) => document.entity.visibility === value).length;
  const hint = (value: EntityVisibility) => (mixed ? `${countOf(value)}개` : null);

  let copied = $state(false);
  let timer: ReturnType<typeof setTimeout> | undefined;

  $effect(() => {
    return () => {
      if (timer) clearTimeout(timer);
    };
  });

  const selectUrl = (event: Event) => {
    (event.currentTarget as HTMLInputElement).select();
  };

  const copyLinks = async () => {
    await navigator.clipboard.writeText(documents.data.map((document) => document.entity.url).join('\n'));
    mixpanel.track('copy_document_share_url', { tab: 'view', count: documents.data.length });

    if (timer) clearTimeout(timer);
    copied = true;
    timer = setTimeout(() => (copied = false), 2000);
  };

  const setVisibility = async (next: EntityVisibility) => {
    if (visibility === next || changeableIds.length === 0) return;
    if (next !== EntityVisibility.PRIVATE && !SubscribeModal.gate('share_document')) return;

    try {
      await updateDocumentsOption({ input: { documentIds: changeableIds, visibility: next } });
      mixpanel.track('update_document_option', { visibility: next, via: 'share_modal', count: changeableIds.length });
    } catch (err) {
      Toast.error(publicationErrorMessage(publicationErrorCode(err)));
    }
  };
</script>

{#snippet linkBody()}
  {#if multiple}
    <Button style={css.raw({ alignSelf: 'flex-start', gap: '6px' })} onclick={copyLinks} size="sm" variant="secondary">
      <Icon style={copied ? css.raw({ color: 'success.default' }) : undefined} icon={copied ? CheckIcon : CopyIcon} size={14} />
      {copied ? '복사되었어요' : `링크 ${documents.data.length}개 복사`}
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
        value={documents.data[0].entity.url}
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
        aria-label="글 보기"
        href={documents.data[0].entity.url}
        rel="noopener noreferrer"
        target="_blank"
        use:tooltip={{ message: '글 보기', placement: 'top' }}
      >
        <Icon icon={ExternalLinkIcon} size={14} />
      </a>
    </div>
  {/if}

  <div class={flex({ flexDirection: 'column', gap: '8px' })}>
    <div class={css({ fontSize: '12px', fontWeight: 'semibold', color: 'text.hint' })}>읽기 설정</div>
    <ReadingSettings documents$key={documents.data} />
  </div>
{/snippet}

<div class={flex({ flexDirection: 'column', gap: '24px', paddingTop: '2px', paddingX: '24px', paddingBottom: '24px' })}>
  <section class={flex({ flexDirection: 'column', gap: '10px' })}>
    <div class={css(groupLabelStyle)}>공개 범위</div>

    <div class={flex({ flexDirection: 'column', gap: '8px' })}>
      <div class={flex({ flexDirection: 'column', gap: '8px' })} aria-label="공개 범위" role="radiogroup">
        <OptionCard
          description="나만 볼 수 있어요."
          disabled={allPublished}
          hint={hint(EntityVisibility.PRIVATE)}
          icon={LockIcon}
          label="비공개"
          onclick={() => setVisibility(EntityVisibility.PRIVATE)}
          selected={visibility === EntityVisibility.PRIVATE}
        />
        <OptionCard
          body={linkBody}
          description="링크가 있는 누구나 볼 수 있어요."
          disabled={allPublished}
          hint={hint(EntityVisibility.UNLISTED)}
          icon={LinkIcon}
          label="링크가 있는 사람"
          onclick={() => setVisibility(EntityVisibility.UNLISTED)}
          selected={visibility === EntityVisibility.UNLISTED}
        />
      </div>

      <OptionCard
        description="스페이스에 글로 올려 누구나 읽을 수 있게 해요."
        icon={SendIcon}
        label="스페이스에 발행"
        onclick={onPublishStep}
        selected={anyPublishing}
        trailing="chevron"
      />
    </div>

    {#if mixed}
      <p class={css({ fontSize: '12px', lineHeight: '[1.5]', color: 'text.hint' })}>
        공개 범위가 서로 달라요. 고르면 글 {changeableIds.length}개에 모두 적용돼요.
      </p>
    {/if}

    {#if allPublished}
      <p class={css({ fontSize: '12px', lineHeight: '[1.5]', color: 'text.hint' })}>공개 방식을 바꾸려면 먼저 발행을 취소해야 해요.</p>
    {:else if publishedCount > 0}
      <p class={css({ fontSize: '12px', lineHeight: '[1.5]', color: 'text.hint' })}>
        발행 중인 글 {publishedCount}개는 공개 방식을 바꾸려면 먼저 발행을 취소해야 해요.
      </p>
    {/if}
  </section>
</div>
