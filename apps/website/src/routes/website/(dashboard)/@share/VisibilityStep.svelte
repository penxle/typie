<script lang="ts">
  import { createFragment, createMutation } from '@mearie/svelte';
  import { EntityVisibility } from '@typie/lib/enums';
  import { css, cx } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { Icon } from '@typie/ui/components';
  import { Toast } from '@typie/ui/notification';
  import mixpanel from 'mixpanel-browser';
  import CheckIcon from '~icons/lucide/check';
  import ChevronRightIcon from '~icons/lucide/chevron-right';
  import CopyIcon from '~icons/lucide/copy';
  import ExternalLinkIcon from '~icons/lucide/external-link';
  import LinkIcon from '~icons/lucide/link';
  import LockIcon from '~icons/lucide/lock';
  import SendIcon from '~icons/lucide/send';
  import { publicationErrorCode } from '$lib/publication/error';
  import { publicationErrorMessage } from '$lib/publication/publish-form';
  import { graphql } from '$mearie';
  import { SubscribeModal } from '../@subscription/subscribe-modal.svelte';
  import { groupLabelStyle, linkFieldButtonStyle, linkFieldInputStyle, linkFieldStyle } from './publish-styles';
  import ReadingSettings from './ReadingSettings.svelte';
  import type { DashboardLayout_Share_VisibilityStep_document$key } from '$mearie';

  type Props = {
    document$key: DashboardLayout_Share_VisibilityStep_document$key;
    onPublishStep: () => void;
  };

  let { document$key, onPublishStep }: Props = $props();

  const document = createFragment(
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
          publishedAt
          scheduledAt
          url
        }

        ...DashboardLayout_Share_ReadingSettings_document
      }
    `),
    () => document$key,
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

  const publication = $derived(document.data.publication);
  const published = $derived(publication?.state === 'PUBLISHED');
  const scheduled = $derived(publication?.state === 'SCHEDULED');
  const visibility = $derived(document.data.entity.visibility);

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

  const copyLink = async () => {
    await navigator.clipboard.writeText(document.data.entity.url);
    mixpanel.track('copy_document_share_url', { tab: 'view', count: 1 });

    if (timer) clearTimeout(timer);
    copied = true;
    timer = setTimeout(() => (copied = false), 2000);
  };

  const setVisibility = async (next: EntityVisibility) => {
    if (visibility === next || published) return;
    if (next !== EntityVisibility.PRIVATE && !SubscribeModal.gate('share_document')) return;

    try {
      await updateDocumentsOption({ input: { documentIds: [document.data.id], visibility: next } });
      mixpanel.track('update_document_option', { visibility: next, via: 'share_modal' });
    } catch (err) {
      Toast.error(publicationErrorMessage(publicationErrorCode(err)));
    }
  };
</script>

{#snippet optionCard(selected: boolean, disabledCard: boolean, head: import('svelte').Snippet, cardBody?: import('svelte').Snippet)}
  <div
    class={css({
      borderWidth: '1px',
      borderColor: selected ? 'accent.default' : 'border.hairline',
      borderRadius: '8px',
      backgroundColor: 'surface.default',
      transition: 'common',
      _hover: { borderColor: selected || disabledCard ? undefined : 'border.emphasis' },
    })}
  >
    {@render head()}

    {#if cardBody}
      <div
        class={flex({
          flexDirection: 'column',
          gap: '14px',
          paddingX: '12px',
          paddingY: '14px',
          borderTopWidth: '1px',
          borderColor: 'border.hairline',
        })}
      >
        {@render cardBody()}
      </div>
    {/if}
  </div>
{/snippet}

{#snippet optionIcon(icon: typeof LockIcon, selected: boolean)}
  <span
    class={center({
      flexShrink: '0',
      size: '32px',
      borderRadius: '8px',
      color: selected ? 'text.default' : 'text.muted',
      backgroundColor: selected ? 'surface.active' : 'surface.inset',
      transition: 'common',
    })}
  >
    <Icon {icon} size={16} />
  </span>
{/snippet}

{#snippet optionRadio(selected: boolean)}
  <span
    class={center({
      flexShrink: '0',
      size: '16px',
      borderWidth: '[1.5px]',
      borderColor: selected ? 'accent.default' : 'border.emphasis',
      borderRadius: 'full',
      transition: 'common',
    })}
  >
    {#if selected}
      <span class={css({ size: '8px', borderRadius: 'full', backgroundColor: 'accent.default' })}></span>
    {/if}
  </span>
{/snippet}

{#snippet option(value: EntityVisibility, icon: typeof LockIcon, label: string, description: string, cardBody?: import('svelte').Snippet)}
  {@const selected = visibility === value}

  {#snippet head()}
    <button
      class={flex({
        alignItems: 'center',
        gap: '12px',
        width: 'full',
        paddingX: '12px',
        paddingY: '11px',
        textAlign: 'left',
        _disabled: { opacity: '45', cursor: 'not-allowed' },
      })}
      aria-checked={selected}
      disabled={published}
      onclick={() => setVisibility(value)}
      role="radio"
      type="button"
    >
      {@render optionIcon(icon, selected)}

      <span class={flex({ flexDirection: 'column', gap: '2px', flex: '1', minWidth: '0' })}>
        <span class={css({ fontSize: '13px', fontWeight: 'semibold' })}>{label}</span>
        <span class={css({ overflow: 'hidden', fontSize: '12px', textOverflow: 'ellipsis', color: 'text.hint' })}>{description}</span>
      </span>

      {@render optionRadio(selected)}
    </button>
  {/snippet}

  {@render optionCard(selected, published, head, selected ? cardBody : undefined)}
{/snippet}

{#snippet linkBody()}
  <div class={css(linkFieldStyle)}>
    <input
      class={css(linkFieldInputStyle)}
      aria-label="공유 링크"
      autocomplete="off"
      onclick={selectUrl}
      onfocus={selectUrl}
      readonly
      spellcheck="false"
      value={document.data.entity.url}
    />

    <button
      class={center(linkFieldButtonStyle)}
      aria-label="링크 복사"
      onclick={copyLink}
      type="button"
      use:tooltip={{ message: copied ? '복사되었어요' : '링크 복사', placement: 'top', keepOnClick: true }}
    >
      <Icon style={copied ? css.raw({ color: 'success.default' }) : undefined} icon={copied ? CheckIcon : CopyIcon} size={14} />
    </button>

    <a
      class={center(linkFieldButtonStyle)}
      aria-label="글 보기"
      href={document.data.entity.url}
      rel="noopener noreferrer"
      target="_blank"
      use:tooltip={{ message: '글 보기', placement: 'top' }}
    >
      <Icon icon={ExternalLinkIcon} size={14} />
    </a>
  </div>

  <div class={flex({ flexDirection: 'column', gap: '8px' })}>
    <div class={css({ fontSize: '12px', fontWeight: 'semibold', color: 'text.hint' })}>읽기 설정</div>
    <ReadingSettings document$key={document.data} />
  </div>
{/snippet}

<div class={flex({ flexDirection: 'column', gap: '24px', paddingTop: '2px', paddingX: '24px', paddingBottom: '24px' })}>
  <section class={flex({ flexDirection: 'column', gap: '10px' })}>
    <div class={css(groupLabelStyle)}>공개 범위</div>

    <div class={flex({ flexDirection: 'column', gap: '8px' })}>
      <div class={flex({ flexDirection: 'column', gap: '8px' })} aria-label="공개 범위" role="radiogroup">
        {@render option(EntityVisibility.PRIVATE, LockIcon, '비공개', '나만 볼 수 있어요.')}
        {@render option(EntityVisibility.UNLISTED, LinkIcon, '링크가 있는 사람', '링크가 있는 누구나 볼 수 있어요.', linkBody)}
      </div>

      {#snippet publishHead()}
        <button
          class={flex({
            alignItems: 'center',
            gap: '12px',
            width: 'full',
            paddingX: '12px',
            paddingY: '11px',
            textAlign: 'left',
            _hover: { '& .publish-chevron': { color: 'text.muted', translateX: '2px' } },
          })}
          onclick={onPublishStep}
          type="button"
        >
          {@render optionIcon(SendIcon, published || scheduled)}

          <span class={flex({ flexDirection: 'column', gap: '2px', flex: '1', minWidth: '0' })}>
            <span class={css({ fontSize: '13px', fontWeight: 'semibold' })}>스페이스에 발행</span>

            <span class={css({ overflow: 'hidden', fontSize: '12px', textOverflow: 'ellipsis', color: 'text.hint' })}>
              스페이스에 글로 올려 누구나 읽을 수 있게 해요.
            </span>
          </span>

          <span class={cx('publish-chevron', center({ flexShrink: '0', color: 'text.hint', translate: 'auto', transition: 'common' }))}>
            <Icon icon={ChevronRightIcon} size={16} />
          </span>
        </button>
      {/snippet}

      {@render optionCard(published || scheduled, false, publishHead)}
    </div>

    {#if published}
      <p class={css({ fontSize: '12px', lineHeight: '[1.5]', color: 'text.hint' })}>공개 방식을 바꾸려면 먼저 발행을 취소해야 해요.</p>
    {/if}
  </section>
</div>
