<script lang="ts">
  import { getChangedRanges } from '@tiptap/core';
  import { Plugin, PluginKey, Transaction } from '@tiptap/pm/state';
  import { Decoration, DecorationSet } from '@tiptap/pm/view';
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { Button, Icon, RingSpinner } from '@typie/ui/components';
  import mixpanel from 'mixpanel-browser';
  import { nanoid } from 'nanoid';
  import { onMount, tick, untrack } from 'svelte';
  import { fly } from 'svelte/transition';
  import { absolutePositionToRelativePosition, relativePositionToAbsolutePosition, ySyncPluginKey } from 'y-prosemirror';
  import CircleAlertIcon from '~icons/lucide/circle-alert';
  import CircleCheckIcon from '~icons/lucide/circle-check';
  import LightbulbIcon from '~icons/lucide/lightbulb';
  import XIcon from '~icons/lucide/x';
  import { pushState } from '$app/navigation';
  import { fragment, graphql } from '$graphql';
  import { getViewContext } from '../@split-view/context.svelte';
  import type { Editor } from '@tiptap/core';
  import type { Ref } from '@typie/ui/utils';
  import type { Editor_Panel_PanelAi_user } from '$graphql';

  type Props = {
    $user: Editor_Panel_PanelAi_user;
    editor?: Ref<Editor>;
  };

  type AiFeedback = {
    id: string;
    from: number;
    to: number;
    relativeFrom: unknown;
    relativeTo: unknown;
    startText: string;
    endText: string;
    feedback: string;
  };

  let { $user: _user, editor }: Props = $props();

  const user = fragment(
    _user,
    graphql(`
      fragment Editor_Panel_PanelAi_user on User {
        id
        preferences

        subscription {
          id
        }
      }
    `),
  );

  const view = getViewContext();
  const aiOptIn = $derived(($user.preferences.aiOptIn as boolean | undefined) ?? false);

  let inflight = $state(false);
  let mounted = $state(false);
  let feedbacks = $state<AiFeedback[]>([]);
  let activeFeedback = $state<AiFeedback>();
  let hasChecked = $state(false);
  let checkFailed = $state(false);
  let listContainer = $state<HTMLElement>();
  let progress = $state<{ current: number; total: number; phase: string } | null>(null);

  const literaryAnalysisStream = graphql(`
    subscription Editor_Panel_Ai_LiteraryAnalysisStream($body: JSON!) {
      literaryAnalysisStream(body: $body) {
        type
        feedback {
          from
          to
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

  const runAnalysis = () => {
    if (!editor?.current || inflight) {
      return;
    }

    if (currentUnsubscribe) {
      currentUnsubscribe();
      currentUnsubscribe = null;
    }

    inflight = true;
    hasChecked = true;
    checkFailed = false;
    feedbacks = [];
    progress = null;

    const body = editor.current.getJSON();
    const { binding } = ySyncPluginKey.getState(editor.current.view.state);

    currentUnsubscribe = literaryAnalysisStream.subscribe({ body }, (payload) => {
      if (payload.type === 'feedback' && payload.feedback) {
        const item = payload.feedback;
        const newFeedback = {
          id: nanoid(),
          from: item.from,
          to: item.to,
          startText: item.startText,
          endText: item.endText,
          feedback: item.feedback,
          relativeFrom: absolutePositionToRelativePosition(item.from, binding.type, binding.mapping),
          relativeTo: absolutePositionToRelativePosition(item.to, binding.type, binding.mapping),
        };
        feedbacks = [...feedbacks, newFeedback].toSorted((a, b) => a.from - b.from);
        scrollToBottom();
      } else if (payload.type === 'progress' && payload.progress) {
        progress = payload.progress;
      } else if (payload.type === 'complete') {
        inflight = false;
        progress = null;
        mixpanel.track('ai-feedback', { feedbacks: feedbacks.length, via: 'panel' });
      } else if (payload.type === 'error') {
        inflight = false;
        progress = null;
        checkFailed = true;
      }
    });
  };

  const scrollToFeedback = (feedback: AiFeedback) => {
    if (!editor?.current) return;

    editor.current
      .chain()
      .setTextSelection({ from: feedback.to, to: feedback.to })
      .scrollIntoViewFixed({ pos: feedback.from, position: editor.current.storage.typewriter.position ?? 0.5, animate: true })
      .run();

    activeFeedback = feedback;
  };

  const handleTransaction = ({ editor, transaction }: { editor: Editor; transaction: Transaction }) => {
    const { binding } = ySyncPluginKey.getState(editor.view.state);

    if (transaction.docChanged) {
      const ranges = getChangedRanges(transaction);
      const meta = transaction.getMeta(ySyncPluginKey);
      const isUndoRedo = meta?.isUndoRedoOperation;

      feedbacks = feedbacks
        .map((feedback) => {
          const from = relativePositionToAbsolutePosition(binding.doc, binding.type, feedback.relativeFrom, binding.mapping);
          const to = relativePositionToAbsolutePosition(binding.doc, binding.type, feedback.relativeTo, binding.mapping);

          if (from === null || to === null) {
            return null;
          }

          if (!isUndoRedo) {
            for (const { newRange } of ranges) {
              if (from <= newRange.to && to >= newRange.from) {
                return null;
              }
            }
          }

          return { ...feedback, from, to };
        })
        .filter((feedback) => feedback !== null);

      if (activeFeedback) {
        const updatedActive = feedbacks.find((f) => f.id === activeFeedback?.id);
        if (updatedActive) {
          activeFeedback = updatedActive;
        } else {
          activeFeedback = undefined;
        }
      }
    }
  };

  $effect(() => {
    if (editor?.current && !mounted) {
      mounted = true;
    }
  });

  $effect(() => {
    void feedbacks;
    void activeFeedback;
    untrack(() => {
      if (editor?.current) {
        editor.current.view.dispatch(editor.current.view.state.tr);
      }
    });
  });

  $effect(() => {
    if (mounted && hasChecked) {
      return untrack(() => {
        const key = new PluginKey('ai-feedback-panel');

        editor?.current.on('transaction', handleTransaction);
        editor?.current.registerPlugin(
          new Plugin({
            key,
            props: {
              decorations: (state) => {
                if (!activeFeedback) {
                  return DecorationSet.empty;
                }

                return DecorationSet.create(state.doc, [
                  Decoration.inline(activeFeedback.from, activeFeedback.to, {
                    class: css({
                      backgroundColor: 'accent.brand.subtle',
                      borderRadius: '2px',
                    }),
                    'data-ai-feedback': activeFeedback.id,
                  }),
                ]);
              },
            },
          }),
        );

        return () => {
          editor?.current.unregisterPlugin(key);
          editor?.current.off('transaction', handleTransaction);
        };
      });
    }
  });

  onMount(() => {
    return () => {
      activeFeedback = undefined;
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
              activeFeedback = undefined;
            } else {
              scrollToFeedback(feedback);
            }
          }}
          onkeydown={(e) => {
            if (e.key === 'Enter' || e.key === ' ') {
              e.preventDefault();
              if (activeFeedback?.id === feedback.id) {
                activeFeedback = undefined;
              } else {
                scrollToFeedback(feedback);
              }
            } else if (e.key === 'ArrowUp') {
              e.preventDefault();
              const currentIndex = feedbacks.findIndex((f) => f.id === feedback.id);
              const prevFeedback = feedbacks[currentIndex - 1];
              if (prevFeedback) {
                scrollToFeedback(prevFeedback);
                const prevElement = document.querySelector(
                  `[data-view-id="${view.id}"] [data-panel-ai-feedback="${prevFeedback.id}"]`,
                ) as HTMLElement;
                prevElement?.focus();
              }
            } else if (e.key === 'ArrowDown') {
              e.preventDefault();
              const currentIndex = feedbacks.findIndex((f) => f.id === feedback.id);
              const nextFeedback = feedbacks[currentIndex + 1];
              if (nextFeedback) {
                scrollToFeedback(nextFeedback);
                const nextElement = document.querySelector(
                  `[data-view-id="${view.id}"] [data-panel-ai-feedback="${nextFeedback.id}"]`,
                ) as HTMLElement;
                nextElement?.focus();
              }
            }
          }}
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
              feedbacks = feedbacks.filter((f) => f.id !== feedback.id);
              if (activeFeedback?.id === feedback.id) {
                activeFeedback = undefined;
              }
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
