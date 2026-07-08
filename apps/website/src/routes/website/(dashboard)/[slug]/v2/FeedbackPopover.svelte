<script lang="ts">
  import { createMutation } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { Icon, Popover } from '@typie/ui/components';
  import { Toast } from '@typie/ui/notification';
  import { tick } from 'svelte';
  import AngryIcon from '~icons/lucide/angry';
  import AnnoyedIcon from '~icons/lucide/annoyed';
  import CheckIcon from '~icons/lucide/check';
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import LaughIcon from '~icons/lucide/laugh';
  import MessageSquareIcon from '~icons/lucide/message-square';
  import SmileIcon from '~icons/lucide/smile';
  import { page } from '$app/state';
  import { graphql } from '$mearie';

  const [submitFeedback] = createMutation(
    graphql(`
      mutation FeedbackPopoverV2_SubmitFeedback_Mutation($input: SubmitFeedbackInput!) {
        submitFeedback(input: $input)
      }
    `),
  );

  const moods = [
    { icon: AngryIcon, value: 'angry' },
    { icon: AnnoyedIcon, value: 'annoyed' },
    { icon: SmileIcon, value: 'good' },
    { icon: LaughIcon, value: 'great' },
  ] as const;

  const topics = [
    { value: 'editor', label: '글쓰기/편집' },
    { value: 'share', label: '발행/공유' },
    { value: 'design', label: '테마/디자인' },
    { value: 'billing', label: '구독/결제' },
    { value: 'other', label: '기타' },
  ];

  let topic = $state('');
  let topicOpen = $state(false);
  let content = $state('');
  let mood = $state<string | null>(null);
  let submitting = $state(false);
  let textareaEl = $state<HTMLTextAreaElement>();

  const selectedTopicLabel = $derived(topics.find((c) => c.value === topic)?.label);
  const canSubmit = $derived(!!topic && !!content.trim() && !submitting);

  let popoverOpen = $state(false);

  const handleSubmit = async () => {
    if (!canSubmit) return;
    submitting = true;
    try {
      await submitFeedback({ input: { topic, content: content.trim(), mood, url: page.url.href } });
      Toast.success('피드백을 보냈어요. 감사해요!');
      topic = '';
      content = '';
      mood = null;
      popoverOpen = false;
    } finally {
      submitting = false;
    }
  };
</script>

<Popover
  style={flex.raw({
    alignItems: 'center',
    gap: '4px',
    paddingX: '8px',
    paddingY: '4px',
    borderRadius: '4px',
    borderWidth: '1px',
    borderColor: 'border.default',
    fontSize: '11px',
    fontWeight: 'semibold',
    whiteSpace: 'nowrap',
    color: 'text.subtle',
    backgroundColor: 'transparent',
    cursor: 'pointer',
    transition: 'common',
    _hover: { backgroundColor: 'surface.muted' },
  })}
  contentStyle={css.raw({ padding: '12px', width: '300px' })}
  onopen={async () => {
    await tick();
    textareaEl?.focus();
  }}
  placement="bottom-end"
  bind:open={popoverOpen}
>
  {#snippet trigger()}
    <Icon icon={MessageSquareIcon} size={12} />
    <span>의견 보내기</span>
  {/snippet}
  <div class={flex({ flexDirection: 'column', gap: '8px' })}>
    <div class={css({ position: 'relative' })}>
      <button
        class={flex({
          width: 'full',
          alignItems: 'center',
          justifyContent: 'space-between',
          padding: '8px',
          fontSize: '13px',
          borderWidth: '1px',
          borderColor: topicOpen ? 'border.brand' : 'border.default',
          borderRadius: '4px',
          backgroundColor: 'surface.default',
          color: selectedTopicLabel ? 'text.default' : 'text.faint',
          cursor: 'pointer',
          transition: 'common',
          _hover: { borderColor: 'border.brand' },
        })}
        onclick={() => {
          topicOpen = !topicOpen;
        }}
        type="button"
      >
        <span>{selectedTopicLabel ?? '주제를 골라주세요...'}</span>
        <Icon
          style={css.raw({
            color: 'text.faint',
            transition: 'common',
            transform: topicOpen ? 'rotate(180deg)' : 'rotate(0deg)',
          })}
          icon={ChevronDownIcon}
          size={14}
        />
      </button>
      {#if topicOpen}
        <div
          class={flex({
            flexDirection: 'column',
            position: 'absolute',
            top: '[calc(100% + 4px)]',
            left: '0',
            width: 'full',
            borderWidth: '1px',
            borderColor: 'border.default',
            borderRadius: '4px',
            backgroundColor: 'surface.default',
            boxShadow: 'small',
            zIndex: '1',
            overflow: 'hidden',
          })}
          data-floating-keep-open
        >
          {#each topics as cat (cat.value)}
            <button
              class={flex({
                justifyContent: 'space-between',
                alignItems: 'center',
                gap: '16px',
                width: 'full',
                paddingX: '10px',
                paddingY: '8px',
                textAlign: 'left',
                fontSize: '13px',
                color: topic === cat.value ? 'text.brand' : 'text.default',
                _hover: { color: 'text.brand', backgroundColor: 'surface.subtle' },
                _focus: { color: 'text.brand', backgroundColor: 'surface.subtle' },
              })}
              onclick={() => {
                topic = cat.value;
                topicOpen = false;
                textareaEl?.focus();
              }}
              type="button"
            >
              {cat.label}
              {#if topic === cat.value}
                <Icon icon={CheckIcon} size={16} />
              {/if}
            </button>
          {/each}
        </div>
      {/if}
    </div>
    <textarea
      bind:this={textareaEl}
      class={css({
        width: 'full',
        minHeight: '80px',
        padding: '8px',
        fontSize: '13px',
        borderWidth: '1px',
        borderColor: 'border.default',
        borderRadius: '4px',
        backgroundColor: 'surface.default',
        color: 'text.default',
        resize: 'none',
        _focus: { borderColor: 'border.brand', outline: 'none' },
        _placeholder: { color: 'text.faint' },
      })}
      onkeydown={(e) => {
        if (!(e.key === 'Enter' && (e.metaKey || e.ctrlKey)) || e.isComposing) {
          return;
        }

        e.preventDefault();
        handleSubmit();
      }}
      placeholder="칭찬도, 불만도, 아이디어도 다 좋아요!"
      bind:value={content}></textarea>
    <div class={flex({ justifyContent: 'space-between', alignItems: 'center' })}>
      <div class={flex({ gap: '2px' })}>
        {#each moods as m (m.value)}
          <button
            class={css({
              padding: '4px',
              borderRadius: 'full',
              color: mood === m.value ? 'accent.brand.default' : 'text.faint',
              backgroundColor: mood === m.value ? 'accent.brand.subtle' : 'transparent',
              boxShadow: mood === m.value ? '[inset 0 0 0 0.5px token(colors.accent.brand.default)]' : '[none]',
              cursor: 'pointer',
              transition: 'common',
              _hover: mood === m.value ? {} : { color: 'text.default' },
            })}
            onclick={() => {
              mood = mood === m.value ? null : m.value;
            }}
            type="button"
          >
            <Icon style={css.raw({ '& *': { strokeWidth: '[1.5px]' } })} icon={m.icon} size={18} />
          </button>
        {/each}
      </div>
      <button
        class={flex({
          alignItems: 'center',
          paddingX: '12px',
          paddingY: '6px',
          borderRadius: '4px',
          fontSize: '13px',
          fontWeight: 'semibold',
          color: 'white',
          backgroundColor: 'accent.brand.default',
          cursor: 'pointer',
          transition: 'common',
          _hover: { backgroundColor: 'accent.brand.hover' },
          _disabled: { opacity: '50', cursor: 'not-allowed', _hover: { backgroundColor: 'accent.brand.default' } },
        })}
        disabled={!canSubmit}
        onclick={handleSubmit}
        type="button"
        use:tooltip={{
          message: topic ? (content.trim() ? null : '내용을 입력해주세요') : '주제를 선택해주세요',
          delay: 0,
          placement: 'left',
        }}
      >
        보내기
      </button>
    </div>
  </div>
</Popover>
