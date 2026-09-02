<script lang="ts">
  import { createMutation } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, Icon, Modal, Tooltip } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import { Toast } from '@typie/ui/notification';
  import AngryIcon from '~icons/lucide/angry';
  import AnnoyedIcon from '~icons/lucide/annoyed';
  import LaughIcon from '~icons/lucide/laugh';
  import SmileIcon from '~icons/lucide/smile';
  import XIcon from '~icons/lucide/x';
  import { page } from '$app/state';
  import { graphql } from '$mearie';

  const app = getAppContext();

  const [submitFeedback] = createMutation(
    graphql(`
      mutation DashboardLayout_FeedbackModal_SubmitFeedback_Mutation($input: SubmitFeedbackInput!) {
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
  let content = $state('');
  let mood = $state<string | null>(null);
  let submitting = $state(false);
  let textareaEl = $state<HTMLTextAreaElement>();

  const canSubmit = $derived(!!topic && !!content.trim() && !submitting);
  const hint = $derived(topic ? (content.trim() ? undefined : '내용을 입력해주세요') : '주제를 선택해주세요');

  const close = () => {
    app.state.feedbackOpen = false;
  };

  const handleSubmit = async () => {
    if (!canSubmit) return;
    submitting = true;
    try {
      await submitFeedback({ input: { topic, content: content.trim(), mood, url: page.url.href } });
    } catch {
      Toast.error('피드백을 보내지 못했어요. 잠시 후 다시 시도해 주세요');
      return;
    } finally {
      submitting = false;
    }

    topic = '';
    content = '';
    mood = null;
    close();
    Toast.success('피드백을 보냈어요. 감사해요!');
  };

  $effect(() => {
    if (app.state.feedbackOpen) {
      setTimeout(() => {
        textareaEl?.focus();
      }, 0);
    }
  });

  const choiceStyle = css.raw({
    borderWidth: '1px',
    borderColor: 'border.default',
    borderRadius: 'full',
    color: 'text.muted',
    backgroundColor: 'surface.default',
    cursor: 'pointer',
    transition: 'common',
    _hover: { borderColor: 'border.strong', color: 'text.default' },
  });

  const choiceSelectedStyle = css.raw({
    borderColor: 'border.strong',
    color: 'text.default',
    backgroundColor: 'surface.muted',
  });
</script>

<Modal style={css.raw({ maxWidth: '400px', padding: '24px' })} onclose={close} open={app.state.feedbackOpen}>
  <div class={flex({ alignItems: 'center', justifyContent: 'space-between', marginBottom: '16px' })}>
    <h2 class={css({ fontSize: '15px', fontWeight: 'bold', letterSpacing: '-0.01em', color: 'text.default' })}>의견 보내기</h2>
    <button
      class={css({
        display: 'flex',
        marginY: '-4px',
        padding: '4px',
        borderRadius: '6px',
        color: 'text.faint',
        cursor: 'pointer',
        transition: 'colors',
        _hover: { color: 'text.subtle', backgroundColor: 'surface.muted' },
      })}
      aria-label="닫기"
      onclick={close}
      type="button"
    >
      <Icon icon={XIcon} size={16} />
    </button>
  </div>

  <div class={flex({ flexDirection: 'column', gap: '12px' })}>
    <div class={flex({ flexWrap: 'wrap', gap: '6px' })} aria-label="주제" role="radiogroup">
      {#each topics as t (t.value)}
        {@const selected = topic === t.value}
        <button
          class={css(choiceStyle, selected && choiceSelectedStyle, {
            paddingX: '12px',
            paddingY: '6px',
            fontSize: '13px',
            fontWeight: 'medium',
          })}
          aria-checked={selected}
          onclick={() => {
            topic = t.value;
            textareaEl?.focus();
          }}
          role="radio"
          type="button"
        >
          {t.label}
        </button>
      {/each}
    </div>

    <textarea
      bind:this={textareaEl}
      class={css({
        width: 'full',
        minHeight: '120px',
        paddingX: '12px',
        paddingY: '10px',
        borderWidth: '1px',
        borderColor: 'border.subtle',
        borderRadius: '8px',
        fontSize: '14px',
        lineHeight: '[1.5]',
        color: 'text.default',
        backgroundColor: 'surface.default',
        resize: 'none',
        transition: 'common',
        _hover: { borderColor: 'border.default' },
        _focus: { outline: 'none', borderColor: 'border.brand' },
        _placeholder: { color: 'text.faint' },
      })}
      aria-label="내용"
      onkeydown={(e) => {
        if (!(e.key === 'Enter' && (e.metaKey || e.ctrlKey)) || e.isComposing) {
          return;
        }

        e.preventDefault();
        handleSubmit();
      }}
      placeholder="칭찬도, 불만도, 아이디어도 다 좋아요!"
      bind:value={content}></textarea>

    <div class={flex({ alignItems: 'center', justifyContent: 'space-between' })}>
      <div class={flex({ gap: '4px' })} aria-label="기분" role="group">
        {#each moods as m (m.value)}
          {@const selected = mood === m.value}
          <button
            class={css(choiceStyle, selected && choiceSelectedStyle, {
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              size: '32px',
            })}
            aria-pressed={selected}
            onclick={() => {
              mood = selected ? null : m.value;
            }}
            type="button"
          >
            <Icon style={css.raw({ '& *': { strokeWidth: '[1.5px]' } })} icon={m.icon} size={18} />
          </button>
        {/each}
      </div>

      <Tooltip enabled={!!hint} message={hint} placement="top">
        <Button disabled={!canSubmit} loading={submitting} onclick={handleSubmit}>보내기</Button>
      </Tooltip>
    </div>
  </div>
</Modal>
