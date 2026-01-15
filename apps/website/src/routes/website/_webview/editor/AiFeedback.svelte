<script lang="ts">
  import { getChangedRanges } from '@tiptap/core';
  import { Plugin, PluginKey } from '@tiptap/pm/state';
  import { Decoration, DecorationSet } from '@tiptap/pm/view';
  import { css } from '@typie/styled-system/css';
  import { nanoid } from 'nanoid';
  import { onMount, untrack } from 'svelte';
  import { absolutePositionToRelativePosition, relativePositionToAbsolutePosition, ySyncPluginKey } from 'y-prosemirror';
  import { graphql } from '$graphql';
  import type { Editor } from '@tiptap/core';
  import type { Transaction } from '@tiptap/pm/state';
  import type { Ref } from '@typie/ui/utils';

  type Props = {
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

  let { editor }: Props = $props();

  let mounted = $state(false);
  let feedbacks = $state<AiFeedback[]>([]);
  let activeFeedbackId = $state<string | null>(null);
  let currentUnsubscribe: (() => void) | null = null;

  const literaryAnalysisStream = graphql(`
    subscription WebViewEditor_LiteraryAnalysisStream_Subscription($body: JSON!) {
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

      if (activeFeedbackId) {
        const activeExists = feedbacks.some((f) => f.id === activeFeedbackId);
        if (!activeExists) {
          activeFeedbackId = null;
        }
      }
    }
  };

  $effect(() => {
    void feedbacks;
    void activeFeedbackId;
    untrack(() => {
      if (editor?.current) {
        editor.current.view.dispatch(editor.current.view.state.tr);
      }
    });
  });

  $effect(() => {
    if (mounted) {
      return untrack(() => {
        const key = new PluginKey('ai-feedback');

        editor?.current.on('transaction', handleTransaction);
        editor?.current.registerPlugin(
          new Plugin({
            key,
            props: {
              decorations: (state) => {
                if (!activeFeedbackId) {
                  return DecorationSet.empty;
                }

                const activeFeedback = feedbacks.find((f) => f.id === activeFeedbackId);
                if (!activeFeedback) {
                  return DecorationSet.empty;
                }

                return DecorationSet.create(state.doc, [
                  Decoration.inline(activeFeedback.from, activeFeedback.to, {
                    class: css({
                      backgroundColor: 'accent.brand.subtle',
                      borderRadius: '2px',
                    }),
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

  $effect(() => {
    if (editor?.current && !mounted) {
      mounted = true;
    }
  });

  onMount(() => {
    window.__webview__?.setProcedure('runAiFeedback', async () => {
      if (!editor?.current) return { success: false };

      if (currentUnsubscribe) {
        currentUnsubscribe();
        currentUnsubscribe = null;
      }

      feedbacks = [];
      activeFeedbackId = null;

      const body = editor.current.getJSON();
      const { binding } = ySyncPluginKey.getState(editor.current.view.state);

      currentUnsubscribe = literaryAnalysisStream.subscribe({ body }, (payload) => {
        if (payload.type === 'feedback' && payload.feedback) {
          const item = payload.feedback;
          const newFeedback: AiFeedback = {
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

          window.__webview__?.emitEvent('aiFeedbackUpdate', {
            type: 'feedback',
            feedback: {
              id: newFeedback.id,
              from: newFeedback.from,
              to: newFeedback.to,
              startText: newFeedback.startText,
              endText: newFeedback.endText,
              feedback: newFeedback.feedback,
            },
          });
        } else if (payload.type === 'progress' && payload.progress) {
          window.__webview__?.emitEvent('aiFeedbackUpdate', {
            type: 'progress',
            progress: payload.progress,
          });
        } else if (payload.type === 'complete') {
          window.__webview__?.emitEvent('aiFeedbackUpdate', {
            type: 'complete',
            feedbackCount: feedbacks.length,
          });
        } else if (payload.type === 'error') {
          window.__webview__?.emitEvent('aiFeedbackUpdate', {
            type: 'error',
          });
        }
      });

      return { success: true };
    });

    window.__webview__?.setProcedure('stopAiFeedback', () => {
      if (currentUnsubscribe) {
        currentUnsubscribe();
        currentUnsubscribe = null;
      }
    });

    window.__webview__?.setProcedure('setAiFeedbackHighlight', ({ id }: { id: string | null }) => {
      activeFeedbackId = id;
    });

    window.__webview__?.setProcedure('scrollToAiFeedback', ({ id }: { id: string }) => {
      if (!editor?.current) return;

      const feedback = feedbacks.find((f) => f.id === id);
      if (!feedback) return;

      const { binding } = ySyncPluginKey.getState(editor.current.view.state);
      const from = relativePositionToAbsolutePosition(binding.doc, binding.type, feedback.relativeFrom, binding.mapping);

      if (from === null) return;

      editor.current.chain().setTextSelection({ from, to: from }).scrollIntoViewFixed({ pos: from, position: 0.5, animate: true }).run();
    });

    window.__webview__?.setProcedure('dismissAiFeedback', ({ id }: { id: string }) => {
      feedbacks = feedbacks.filter((f) => f.id !== id);
      if (activeFeedbackId === id) {
        activeFeedbackId = null;
      }
    });

    window.__webview__?.setProcedure('clearAiFeedbacks', () => {
      feedbacks = [];
      activeFeedbackId = null;
    });

    return () => {
      if (currentUnsubscribe) {
        currentUnsubscribe();
        currentUnsubscribe = null;
      }
    };
  });
</script>
