<script lang="ts">
  import { posToDOMRect } from '@tiptap/core';
  import { Plugin, PluginKey } from '@tiptap/pm/state';
  import { Decoration, DecorationSet } from '@tiptap/pm/view';
  import { css } from '@typie/styled-system/css';
  import mixpanel from 'mixpanel-browser';
  import { nanoid } from 'nanoid';
  import { onMount, untrack } from 'svelte';
  import { absolutePositionToRelativePosition, relativePositionToAbsolutePosition, ySyncPluginKey } from 'y-prosemirror';
  import { graphql } from '$graphql';
  import type { Editor } from '@tiptap/core';
  import type { Transaction } from '@tiptap/pm/state';
  import type { Ref } from '$lib/utils';

  type Props = {
    editor?: Ref<Editor>;
  };

  type SpellcheckError = {
    id: string;
    from: number;
    to: number;
    relativeFrom: unknown;
    relativeTo: unknown;
    context: string;
    corrections: string[];
    explanation: string;
  };

  let { editor }: Props = $props();

  let mounted = $state(false);
  let errors = $state<SpellcheckError[]>([]);

  const checkSpelling = graphql(`
    mutation WebViewEditor_CheckSpelling_Mutation($input: CheckSpellingInput!) {
      checkSpelling(input: $input) {
        from
        to
        context
        corrections
        explanation
      }
    }
  `);

  const handleTransaction = ({ editor, transaction }: { editor: Editor; transaction: Transaction }) => {
    const { binding } = ySyncPluginKey.getState(editor.view.state);

    if (transaction.docChanged) {
      const changedRanges: { from: number; to: number }[] = [];
      transaction.steps.forEach((_step, index) => {
        const map = transaction.mapping.maps[index];
        if (map) {
          map.forEach((_oldStart, _oldEnd, newStart, newEnd) => {
            changedRanges.push({ from: newStart, to: newEnd });
          });
        }
      });

      errors = errors
        .map((error) => {
          const from = relativePositionToAbsolutePosition(binding.doc, binding.type, error.relativeFrom, binding.mapping);
          const to = relativePositionToAbsolutePosition(binding.doc, binding.type, error.relativeTo, binding.mapping);

          if (from === null || to === null) {
            return null;
          }

          for (const range of changedRanges) {
            if (from <= range.to && to >= range.from) {
              return null;
            }
          }

          return { ...error, from, to };
        })
        .filter((error) => error !== null);
    }
  };

  $effect(() => {
    void errors;
    untrack(() => {
      if (editor?.current) {
        editor.current.view.dispatch(editor.current.view.state.tr);
      }
    });
  });

  $effect(() => {
    if (mounted) {
      return untrack(() => {
        const key = new PluginKey('spellcheck');

        editor?.current.on('transaction', handleTransaction);
        editor?.current.registerPlugin(
          new Plugin({
            key,
            props: {
              decorations: (state) => {
                return DecorationSet.create(
                  state.doc,
                  errors.map((error) =>
                    Decoration.inline(error.from, error.to, {
                      class: css({
                        textDecoration: 'underline',
                        textDecorationColor: 'text.danger',
                        textDecorationStyle: 'wavy',
                        textUnderlineOffset: '2px',
                      }),
                    }),
                  ),
                );
              },
              handleDOMEvents: {
                pointerdown: (view, event) => {
                  const error = errors.find((error) => {
                    const rect = posToDOMRect(view, error.from, error.to);

                    return (
                      rect.left <= event.clientX && rect.right >= event.clientX && rect.top <= event.clientY && rect.bottom >= event.clientY
                    );
                  });
                  if (!error) return false;

                  event.preventDefault();

                  const parser = new DOMParser();
                  const doc = parser.parseFromString(error.explanation, 'text/html');
                  // eslint-disable-next-line unicorn/prefer-dom-node-text-content
                  const explanation = doc.documentElement.innerText;

                  (document.activeElement as HTMLElement)?.blur();
                  window.__webview__?.emitEvent('spellcheckErrorClick', {
                    id: error.id,
                    context: error.context,
                    corrections: error.corrections,
                    explanation,
                  });

                  return true;
                },
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
    window.__webview__?.setProcedure('checkSpelling', async () => {
      if (!editor?.current) return;

      const body = editor.current.getJSON();
      const resp = await checkSpelling({ body });

      mixpanel.track('spellcheck', { errors: resp.length });

      const { binding } = ySyncPluginKey.getState(editor.current.view.state);
      errors = resp.map((error) => ({
        id: nanoid(),
        ...error,
        relativeFrom: absolutePositionToRelativePosition(error.from, binding.type, binding.mapping),
        relativeTo: absolutePositionToRelativePosition(error.to, binding.type, binding.mapping),
      }));

      const parser = new DOMParser();

      return {
        errors: errors.map((error) => {
          const doc = parser.parseFromString(error.explanation, 'text/html');
          // eslint-disable-next-line unicorn/prefer-dom-node-text-content
          const explanation = doc.documentElement.innerText;

          return {
            id: error.id,
            context: error.context,
            corrections: error.corrections,
            explanation,
          };
        }),
      };
    });

    const getErrorPosition = (errorId: string) => {
      if (!editor?.current) return null;

      const error = errors.find((err) => err.id === errorId);
      if (!error) return null;

      const { binding } = ySyncPluginKey.getState(editor.current.view.state);
      const from = relativePositionToAbsolutePosition(binding.doc, binding.type, error.relativeFrom, binding.mapping);
      const to = relativePositionToAbsolutePosition(binding.doc, binding.type, error.relativeTo, binding.mapping);

      return { error, from, to };
    };

    const scrollToPosition = () => {
      setTimeout(() => {
        editor?.current?.commands.scrollIntoViewFixed({
          animate: true,
          position: 0.25,
        });
      }, 0);
    };

    type ApplySpellcheckCorrectionData = { id: string; correction: string };
    window.__webview__?.setProcedure('applySpellcheckCorrection', async ({ id, correction }: ApplySpellcheckCorrectionData) => {
      const position = getErrorPosition(id);
      if (!position || position.from === null || position.to === null || !editor?.current) return;

      editor.current.chain().setTextSelection({ from: position.from, to: position.to }).insertContent(correction).run();
      scrollToPosition();

      errors = errors.filter((err) => err.id !== id);
    });

    type ScrollToSpellcheckErrorData = { id: string };
    window.__webview__?.setProcedure('scrollToSpellcheckError', async ({ id }: ScrollToSpellcheckErrorData) => {
      const position = getErrorPosition(id);
      if (!position || position.from === null || !editor?.current) return;

      editor.current.chain().setTextSelection(position.from).run();
      scrollToPosition();
    });
  });
</script>
