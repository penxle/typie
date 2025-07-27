<script lang="ts">
  import { Plugin, PluginKey } from '@tiptap/pm/state';
  import { Decoration, DecorationSet } from '@tiptap/pm/view';
  import mixpanel from 'mixpanel-browser';
  import { nanoid } from 'nanoid';
  import { onMount, untrack } from 'svelte';
  import { absolutePositionToRelativePosition, relativePositionToAbsolutePosition, ySyncPluginKey } from 'y-prosemirror';
  import { graphql } from '$graphql';
  import { css } from '$styled-system/css';
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
      errors = errors
        .map((error) => {
          const from = relativePositionToAbsolutePosition(binding.doc, binding.type, error.relativeFrom, binding.mapping);
          const to = relativePositionToAbsolutePosition(binding.doc, binding.type, error.relativeTo, binding.mapping);

          if (from === null || to === null) {
            return null;
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
                  const pos = view.posAtCoords({ left: event.clientX, top: event.clientY });
                  if (!pos) return false;

                  const error = errors.find((error) => error.from <= pos.pos && error.to >= pos.pos);
                  if (!error) return false;

                  event.preventDefault();

                  const parser = new DOMParser();
                  const doc = parser.parseFromString(error.explanation, 'text/html');
                  const explanation = doc.documentElement.textContent;

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
          const explanation = doc.documentElement.textContent;

          return {
            id: error.id,
            context: error.context,
            corrections: error.corrections,
            explanation,
          };
        }),
      };
    });

    type ApplySpellcheckCorrectionData = { id: string; correction: string };
    window.__webview__?.setProcedure('applySpellcheckCorrection', async ({ id, correction }: ApplySpellcheckCorrectionData) => {
      if (!editor?.current) return;

      const error = errors.find((err) => err.id === id);
      if (!error) return;

      const { binding } = ySyncPluginKey.getState(editor.current.view.state);
      const from = relativePositionToAbsolutePosition(binding.doc, binding.type, error.relativeFrom, binding.mapping);
      const to = relativePositionToAbsolutePosition(binding.doc, binding.type, error.relativeTo, binding.mapping);

      if (from === null || to === null) return;

      editor.current.chain().setTextSelection({ from, to }).insertContent(correction).run();
      setTimeout(() => {
        editor.current.commands.scrollIntoViewFixed();
      }, 0);

      errors = errors.filter((err) => err.id !== id);
    });
  });
</script>
