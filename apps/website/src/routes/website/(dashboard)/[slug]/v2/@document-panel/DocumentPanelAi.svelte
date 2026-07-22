<script lang="ts">
  import { createFragment, createSubscription } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { Button, Icon, RingSpinner } from '@typie/ui/components';
  import { Toast } from '@typie/ui/notification';
  import { onMount, tick } from 'svelte';
  import { fly } from 'svelte/transition';
  import CircleAlertIcon from '~icons/lucide/circle-alert';
  import CircleCheckIcon from '~icons/lucide/circle-check';
  import LightbulbIcon from '~icons/lucide/lightbulb';
  import XIcon from '~icons/lucide/x';
  import { pushState } from '$app/navigation';
  import { graphql } from '$mearie';
  import type { Editor } from '$lib/editor-ffi/editor.svelte';
  import type { DocumentPanelV2_Ai_document$key, DocumentPanelV2_Ai_user$key } from '$mearie';

  type Props = {
    document$key: DocumentPanelV2_Ai_document$key;
    user$key: DocumentPanelV2_Ai_user$key;
    editor: Editor | undefined;
  };

  let { document$key, user$key, editor }: Props = $props();

  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  const document = createFragment(
    graphql(`
      fragment DocumentPanelV2_Ai_document on Document {
        id
      }
    `),
    () => document$key,
  );

  const user = createFragment(
    graphql(`
      fragment DocumentPanelV2_Ai_user on User {
        id
        preferences

        subscription {
          id
        }
      }
    `),
    () => user$key,
  );

  const aiOptIn = $derived((user.data.preferences.aiOptIn as boolean | undefined) ?? false);

  let inflight = $state(false);
  let hasChecked = $state(false);
  let checkFailed = $state(false);
  let listContainer = $state<HTMLElement>();
  let progress = $state<{ current: number; total: number; phase: string } | null>(null);
  let analysisText = $state<string | null>(null);

  createSubscription(
    graphql(`
      subscription DocumentPanelV2_Ai_LiteraryAnalysisDocumentStreamV2($text: String!) {
        literaryAnalysisDocumentStreamV2(text: $text) {
          type
          feedback {
            id
            start
            end
            startText
            endText
            feedback
            category
          }
          progress {
            current
            total
            phase
          }
        }
      }
    `),
    () => ({ text: analysisText ?? '' }),
    () => ({
      skip: !analysisText,
      onData: (data) => {
        if (!editor || !analysisText || !inflight) return;
        if (editor.proseTextAnnotated() !== analysisText) {
          cancelAnalysisForDocumentEdit();
          return;
        }

        const payload = data.literaryAnalysisDocumentStreamV2;
        if (payload.type === 'feedback' && payload.feedback) {
          const fb = payload.feedback;
          const selection = editor.proseToSelectionAnnotated(fb.start, fb.end);
          if (!selection) return;
          editor.addAiFeedback({
            id: fb.id,
            selection,
            startText: fb.startText,
            endText: fb.endText,
            feedback: fb.feedback,
            category: fb.category ?? null,
          });
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
      onError: () => {
        if (!analysisText || !inflight) return;
        inflight = false;
        progress = null;
        checkFailed = true;
      },
    }),
  );

  const scrollToBottom = async () => {
    if (!listContainer) return;

    const isAtBottom = listContainer.scrollHeight - listContainer.scrollTop - listContainer.clientHeight < 100;
    await tick();
    if (isAtBottom) {
      listContainer.scrollTop = listContainer.scrollHeight;
    }
  };

  const runAnalysis = async () => {
    if (!editor || inflight) return;

    inflight = true;
    hasChecked = true;
    checkFailed = false;
    progress = null;

    editor.clearAiFeedbacks();
    editor.installAiFeedbackDecorations();

    const text = editor.proseTextAnnotated();
    if (!text.trim()) {
      inflight = false;
      analysisText = null;
      return;
    }

    analysisText = null;
    await tick();
    analysisText = text;
  };

  const cancelAnalysis = () => {
    analysisText = null;
    hasChecked = (editor?.aiFeedbacks.length ?? 0) > 0;
    inflight = false;
    progress = null;
  };

  const cancelAnalysisForDocumentEdit = () => {
    if (!inflight) return;
    analysisText = null;
    inflight = false;
    hasChecked = false;
    checkFailed = false;
    progress = null;
    editor?.clearAiFeedbacks();
    Toast.success('내용이 수정되어 분석이 취소됐어요.');
  };

  const activeFeedback = $derived(
    editor && editor.activeAiFeedbackId ? editor.aiFeedbacks.find((f) => f.id === editor.activeAiFeedbackId) : undefined,
  );

  const setActiveFeedback = (feedbackId: string) => {
    editor?.setActiveAiFeedback(feedbackId);
  };

  const removeFeedback = (feedbackId: string) => {
    editor?.removeAiFeedback(feedbackId);
  };

  const handleKeyDown = (e: KeyboardEvent, feedbackId: string) => {
    if (!editor) return;
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      setActiveFeedback(feedbackId);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      const idx = editor.aiFeedbacks.findIndex((f) => f.id === feedbackId);
      const prev = editor.aiFeedbacks[idx - 1];
      if (prev) {
        setActiveFeedback(prev.id);
        const el = globalThis.document.querySelector(`[data-panel-ai-feedback="${prev.id}"]`) as HTMLElement | null;
        el?.focus();
      }
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      const idx = editor.aiFeedbacks.findIndex((f) => f.id === feedbackId);
      const next = editor.aiFeedbacks[idx + 1];
      if (next) {
        setActiveFeedback(next.id);
        const el = globalThis.document.querySelector(`[data-panel-ai-feedback="${next.id}"]`) as HTMLElement | null;
        el?.focus();
      }
    }
  };

  $effect(() => {
    if (activeFeedback) {
      const el = listContainer?.querySelector(`[data-panel-ai-feedback="${activeFeedback.id}"]`) as HTMLElement | null;
      el?.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
    }
  });

  $effect(() => {
    const activeEditor = editor;
    if (!activeEditor) return;

    return activeEditor.on('state_changed', (_, { fields }) => {
      if (fields.includes('doc')) {
        cancelAnalysisForDocumentEdit();
      }
    });
  });

  onMount(() => {
    return () => {
      analysisText = null;
      editor?.clearAiFeedbacks();
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
      {#if editor && hasChecked && !checkFailed && editor.aiFeedbacks.length > 0}
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
          {editor.aiFeedbacks.length}
        </div>
      {/if}
    </div>

    {#if inflight}
      <button
        class={css({
          fontSize: '13px',
          fontWeight: 'medium',
          color: 'text.faint',
          transition: 'common',
          _hover: { color: 'text.subtle' },
        })}
        onclick={cancelAnalysis}
        type="button"
      >
        취소
      </button>
    {:else if hasChecked && aiOptIn}
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
  {:else if (hasChecked && checkFailed) || !user.data.subscription}
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
      {#if editor && !inflight && editor.aiFeedbacks.length === 0}
        <div class={flex({ flexDirection: 'column', alignItems: 'center', gap: '8px', paddingY: '24px' })}>
          <Icon style={css.raw({ color: 'text.faint' })} icon={CircleCheckIcon} size={32} />
          <div class={css({ fontSize: '16px', color: 'text.faint' })}>피드백이 없습니다</div>
        </div>
      {/if}

      {#each editor?.aiFeedbacks ?? [] as feedback (feedback.id)}
        <div
          class={css({
            position: 'relative',
            borderWidth: '1px',
            borderColor: activeFeedback?.id === feedback.id ? 'accent.brand.default!' : 'border.default',
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
            if (activeFeedback?.id === feedback.id) {
              editor?.focus();
            } else {
              setActiveFeedback(feedback.id);
            }
          }}
          onkeydown={(e) => handleKeyDown(e, feedback.id)}
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
            {#if feedback.category}
              <div
                class={css({
                  alignSelf: 'flex-start',
                  borderRadius: '4px',
                  paddingX: '6px',
                  paddingY: '2px',
                  fontSize: '11px',
                  fontWeight: 'semibold',
                  color: 'text.subtle',
                  backgroundColor: 'surface.muted',
                })}
              >
                {feedback.category}
              </div>
            {/if}

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
                lineClamp: activeFeedback?.id === feedback.id ? 'none' : '2',
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
              {:else if progress.phase === 'meta'}
                작품 전체 분석 중...
              {:else if progress.phase === 'analyzing'}
                피드백 중... ({progress.current}/{progress.total})
              {:else}
                준비 중...
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
