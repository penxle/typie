<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { Button, Icon, RingSpinner } from '@typie/ui/components';
  import { nanoid } from 'nanoid';
  import { onMount, tick } from 'svelte';
  import { fly } from 'svelte/transition';
  import CircleAlertIcon from '~icons/lucide/circle-alert';
  import CircleCheckIcon from '~icons/lucide/circle-check';
  import LightbulbIcon from '~icons/lucide/lightbulb';
  import XIcon from '~icons/lucide/x';
  import { pushState } from '$app/navigation';
  import { fragment, graphql } from '$graphql';
  import type { DocumentPanel_Ai_document, DocumentPanel_Ai_user } from '$graphql';
  import type { Editor } from '$lib/editor/editor.svelte';
  import type { AiFeedbackData } from '$lib/editor/types';

  type Props = {
    $document: DocumentPanel_Ai_document;
    $user: DocumentPanel_Ai_user;
    editor: Editor;
  };

  let { $document: _document, $user: _user, editor }: Props = $props();

  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  const document = fragment(
    _document,
    graphql(`
      fragment DocumentPanel_Ai_document on Document {
        id
      }
    `),
  );

  const user = fragment(
    _user,
    graphql(`
      fragment DocumentPanel_Ai_user on User {
        id
        preferences

        subscription {
          id
        }
      }
    `),
  );

  const aiOptIn = $derived(($user.preferences.aiOptIn as boolean | undefined) ?? false);

  let inflight = $state(false);
  let mounted = $state(false);
  let hasChecked = $state(false);
  let checkFailed = $state(false);
  let listContainer = $state<HTMLElement>();
  let progress = $state<{ current: number; total: number; phase: string } | null>(null);

  const feedbacks = $derived(editor.fullAiFeedbackItems);
  const activeItemId = $derived(editor.activeAiFeedbackItemId);

  const literaryAnalysisDocumentStream = graphql(`
    subscription DocumentPanel_Ai_LiteraryAnalysisDocumentStream($text: String!, $mappings: [DocumentTextMappingInput!]!) {
      literaryAnalysisDocumentStream(text: $text, mappings: $mappings) {
        type
        feedback {
          nodeId
          startOffset
          endOffset
          startText
          endText
          feedback
        }
        progress {
          current
          total
          phase
        }
      }
    }
  `);

  let currentUnsubscribe: (() => void) | null = null;

  const scrollToBottom = async () => {
    if (!listContainer) return;

    const isAtBottom = listContainer.scrollHeight - listContainer.scrollTop - listContainer.clientHeight < 100;
    await tick();
    if (isAtBottom) {
      listContainer.scrollTop = listContainer.scrollHeight;
    }
  };

  const updateOverlays = () => {
    editor.setTrackedItems(
      1,
      editor.fullAiFeedbackItems.map((e) => ({
        id: e.id,
        nodeId: e.nodeId,
        startOffset: e.startOffset,
        endOffset: e.endOffset,
      })),
    );
  };

  const runAnalysis = async () => {
    if (!editor || inflight) {
      return;
    }

    await editor.ready;

    const spellcheckData = editor.getTextWithMappings();
    if (!spellcheckData?.text?.trim()) {
      return;
    }

    if (currentUnsubscribe) {
      currentUnsubscribe();
      currentUnsubscribe = null;
    }

    inflight = true;
    hasChecked = true;
    checkFailed = false;
    editor.fullAiFeedbackItems = [];
    editor.setTrackedItems(1, []);
    progress = null;

    currentUnsubscribe = literaryAnalysisDocumentStream.subscribe(
      { text: spellcheckData.text, mappings: spellcheckData.mappings },
      (payload) => {
        if (payload.type === 'feedback' && payload.feedback) {
          const item = payload.feedback;
          const newItem: AiFeedbackData = {
            id: nanoid(),
            nodeId: item.nodeId,
            startOffset: item.startOffset,
            endOffset: item.endOffset,
            startText: item.startText,
            endText: item.endText,
            feedback: item.feedback,
          };
          editor.fullAiFeedbackItems = [...editor.fullAiFeedbackItems, newItem];
          updateOverlays();
          scrollToBottom();
        } else if (payload.type === 'progress' && payload.progress) {
          progress = payload.progress;
        } else if (payload.type === 'complete') {
          inflight = false;
          progress = null;
        } else if (payload.type === 'error') {
          inflight = false;
          progress = null;
          checkFailed = true;
        }
      },
    );
  };

  const scrollToFeedback = (feedback: AiFeedbackData) => {
    if (!editor) return;
    editor.activeAiFeedbackItemId = feedback.id;
    editor.scrollTrackedItemIntoView(feedback.id);
  };

  const removeFeedback = (feedbackId: string) => {
    editor.fullAiFeedbackItems = editor.fullAiFeedbackItems.filter((f) => f.id !== feedbackId);
    updateOverlays();
  };

  const handleKeyDown = (e: KeyboardEvent, feedback: AiFeedbackData) => {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      if (activeItemId !== feedback.id) {
        scrollToFeedback(feedback);
      }
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      const currentIndex = feedbacks.findIndex((f) => f.id === feedback.id);
      const prevFeedback = feedbacks[currentIndex - 1];
      if (prevFeedback) {
        scrollToFeedback(prevFeedback);
        const prevElement = globalThis.document.querySelector(`[data-panel-ai-feedback="${prevFeedback.id}"]`) as HTMLElement;
        prevElement?.focus();
      }
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      const currentIndex = feedbacks.findIndex((f) => f.id === feedback.id);
      const nextFeedback = feedbacks[currentIndex + 1];
      if (nextFeedback) {
        scrollToFeedback(nextFeedback);
        const nextElement = globalThis.document.querySelector(`[data-panel-ai-feedback="${nextFeedback.id}"]`) as HTMLElement;
        nextElement?.focus();
      }
    }
  };

  $effect(() => {
    if (editor && !mounted) {
      mounted = true;
    }
  });

  $effect(() => {
    if (activeItemId) {
      const el = listContainer?.querySelector(`[data-panel-ai-feedback="${activeItemId}"]`) as HTMLElement | null;
      el?.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
    }
  });

  $effect(() => {
    return () => {
      editor?.setTrackedItems(1, []);
    };
  });

  onMount(() => {
    return () => {
      if (currentUnsubscribe) {
        currentUnsubscribe();
        currentUnsubscribe = null;
      }
    };
  });
</script>

<div
  class={flex({
    flexDirection: 'column',
    minWidth: 'var(--min-width)',
    width: 'var(--width)',
    maxWidth: 'var(--max-width)',
    height: 'full',
  })}
>
  <div
    class={flex({
      flexShrink: '0',
      justifyContent: 'space-between',
      alignItems: 'center',
      height: '41px',
      paddingX: '20px',
      fontSize: '13px',
      fontWeight: 'semibold',
      color: 'text.subtle',
      borderBottomWidth: '1px',
      borderColor: 'surface.muted',
    })}
  >
    <div class={flex({ alignItems: 'center', gap: '6px' })}>
      AI 피드백
      {#if hasChecked && !checkFailed && feedbacks.length > 0}
        <div
          class={css({
            borderRadius: '4px',
            paddingX: '6px',
            paddingY: '2px',
            fontSize: '11px',
            fontWeight: 'semibold',
            color: 'accent.brand.default',
            backgroundColor: 'accent.brand.subtle',
          })}
        >
          {feedbacks.length}
        </div>
      {/if}
    </div>

    {#if !inflight && hasChecked && aiOptIn}
      <button
        class={css({
          fontSize: '13px',
          fontWeight: 'medium',
          color: 'text.faint',
          transition: 'common',
          _hover: { color: 'text.subtle' },
        })}
        onclick={runAnalysis}
        type="button"
      >
        다시 분석
      </button>
    {/if}
  </div>

  {#if !aiOptIn}
    <div
      class={flex({
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: 'center',
        gap: '20px',
        paddingY: '60px',
      })}
    >
      <div
        class={center({
          size: '64px',
          borderRadius: '16px',
          backgroundColor: 'surface.muted',
          color: 'text.faint',
        })}
      >
        <Icon icon={LightbulbIcon} size={28} />
      </div>

      <div class={flex({ flexDirection: 'column', alignItems: 'center', gap: '8px' })}>
        <p class={css({ fontSize: '13px', color: 'text.faint', textAlign: 'center' })}>
          AI 기능을 사용하려면
          <br />
          설정에서 활성화해주세요
        </p>
      </div>

      <Button onclick={() => pushState('', { shallowRoute: '/preference/ai' })} size="sm" variant="secondary">설정으로 이동</Button>
    </div>
  {:else if !hasChecked && !inflight}
    <div
      class={flex({
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: 'center',
        gap: '20px',
        paddingY: '60px',
      })}
    >
      <div
        class={center({
          size: '64px',
          borderRadius: '16px',
          backgroundColor: 'surface.muted',
          color: 'text.faint',
        })}
      >
        <Icon icon={LightbulbIcon} size={28} />
      </div>

      <div class={flex({ flexDirection: 'column', alignItems: 'center', gap: '8px' })}>
        <p class={css({ fontSize: '13px', color: 'text.faint', textAlign: 'center' })}>
          글에 대한 AI 피드백을
          <br />
          받아보세요
        </p>
      </div>

      <Button onclick={runAnalysis} size="sm" variant="secondary">분석 시작</Button>
    </div>
  {:else if (hasChecked && checkFailed) || !$user.subscription}
    <div class={flex({ flexDirection: 'column', alignItems: 'center', gap: '8px', paddingY: '40px' })}>
      <Icon style={css.raw({ color: 'text.faint' })} icon={CircleAlertIcon} size={32} />
      <div class={css({ fontSize: '16px', color: 'text.faint' })}>분석에 실패했습니다</div>
      <div class={css({ fontSize: '14px', color: 'text.faint' })}>잠시 후 다시 시도해주세요</div>
    </div>
  {:else if hasChecked || inflight}
    <div
      bind:this={listContainer}
      class={flex({
        flexDirection: 'column',
        gap: '12px',
        paddingX: '12px',
        paddingTop: '16px',
        paddingBottom: '100px',
        overflowY: 'auto',
      })}
    >
      {#if !inflight && feedbacks.length === 0}
        <div class={flex({ flexDirection: 'column', alignItems: 'center', gap: '8px', paddingY: '24px' })}>
          <Icon style={css.raw({ color: 'text.faint' })} icon={CircleCheckIcon} size={32} />
          <div class={css({ fontSize: '16px', color: 'text.faint' })}>피드백이 없습니다</div>
        </div>
      {/if}

      {#each feedbacks as feedback (feedback.id)}
        <div
          class={css({
            position: 'relative',
            borderWidth: '1px',
            borderColor: activeItemId === feedback.id ? 'accent.brand.default!' : 'border.default',
            borderRadius: '8px',
            padding: '12px',
            cursor: 'pointer',
            transition: 'common',
            _hover: {
              borderColor: 'border.strong',
              backgroundColor: 'surface.subtle',
            },
            _focusVisible: {
              borderColor: 'border.strong',
              backgroundColor: 'surface.subtle',
            },
          })}
          data-panel-ai-feedback={feedback.id}
          onclick={() => {
            if (activeItemId === feedback.id) {
              editor.focus();
            } else {
              scrollToFeedback(feedback);
            }
          }}
          onkeydown={(e) => handleKeyDown(e, feedback)}
          role="button"
          tabindex="0"
          in:fly={{ y: 8, duration: 200 }}
        >
          <button
            class={css({
              position: 'absolute',
              top: '8px',
              right: '8px',
              padding: '4px',
              borderRadius: '4px',
              color: 'text.faint',
              transition: 'common',
              _hover: {
                backgroundColor: 'interactive.hover',
                color: 'text.subtle',
              },
              _focusVisible: {
                backgroundColor: 'interactive.hover',
                color: 'text.subtle',
              },
            })}
            onclick={(e) => {
              e.stopPropagation();
              removeFeedback(feedback.id);
            }}
            type="button"
            use:tooltip={{
              message: '무시하기',
              placement: 'top',
            }}
          >
            <Icon icon={XIcon} size={14} />
          </button>

          <div class={flex({ flexDirection: 'column', gap: '8px', paddingRight: '24px' })}>
            <div class={css({ fontSize: '14px', color: 'text.default' })}>
              {#if feedback.startText === feedback.endText}
                "{feedback.startText}"
              {:else}
                "{feedback.startText}" ... "{feedback.endText}"
              {/if}
            </div>

            <div
              class={css({
                fontSize: '12px',
                color: 'text.faint',
                lineClamp: activeItemId === feedback.id ? 'none' : '2',
              })}
            >
              {feedback.feedback}
            </div>
          </div>
        </div>
      {/each}

      {#if inflight}
        <div class={flex({ justifyContent: 'center', alignItems: 'center', gap: '8px', paddingY: '16px' })}>
          <RingSpinner style={css.raw({ size: '16px', color: 'text.faint' })} />
          <div class={css({ fontSize: '13px', color: 'text.faint' })}>
            {#if progress}
              {#if progress.phase === 'summarizing'}
                분석 중... ({progress.current}/{progress.total})
              {:else}
                피드백 중... ({progress.current}/{progress.total})
              {/if}
            {:else}
              준비 중...
            {/if}
          </div>
        </div>
      {/if}
    </div>
  {/if}
</div>
