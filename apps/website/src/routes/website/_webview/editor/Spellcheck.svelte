<script lang="ts">
  import { Mapping } from '@tiptap/pm/transform';
  import mixpanel from 'mixpanel-browser';
  import { untrack } from 'svelte';
  import { fragment, graphql } from '$graphql';
  import { createSpellcheckPlugin, decodeHtmlEntities, mapErrors, spellcheckKey, updateErrorPositions } from '$lib/editor/spellcheck';
  import type { Editor } from '@tiptap/core';
  import type { Transaction } from '@tiptap/pm/state';
  import type { WebViewEditor_Spellcheck_query } from '$graphql';
  import type { SpellingError } from '$lib/editor/spellcheck';
  import type { Ref } from '$lib/utils';

  type Props = {
    $query: WebViewEditor_Spellcheck_query;
    editor?: Ref<Editor>;
  };

  let { $query: _query, editor }: Props = $props();

  const query = fragment(
    _query,
    graphql(`
      fragment WebViewEditor_Spellcheck_query on Query {
        me @required {
          id

          subscription {
            id

            plan {
              id
            }
          }
        }
      }
    `),
  );

  let spellcheckErrors = $state<SpellingError[]>([]);
  let spellcheckMapping = $state<Mapping | undefined>();

  const hasSubscription = $derived(!!$query.me.subscription?.plan);

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

  const spellcheckPlugin = createSpellcheckPlugin(spellcheckKey, {
    onErrorClick: (pos) => {
      const foundError = spellcheckErrors.find((err) => pos >= err.from && pos <= err.to);
      if (foundError) {
        window.__webview__?.emitEvent('spellcheckErrorClick', foundError);
      }
    },
  });

  const handleTransaction = ({ transaction, editor }: { transaction: Transaction; editor: Editor }) => {
    if (transaction.docChanged) {
      if (spellcheckMapping) {
        spellcheckMapping.appendMapping(transaction.mapping);
      } else if (spellcheckErrors.length > 0) {
        spellcheckMapping = new Mapping();
        spellcheckMapping.appendMapping(transaction.mapping);
      } else {
        return;
      }

      spellcheckErrors = updateErrorPositions(spellcheckErrors, transaction);

      editor.commands.command(({ tr }) => {
        tr.setMeta(spellcheckKey, spellcheckErrors);
        tr.setMeta('addToHistory', false);
        return true;
      });
    }
  };

  $effect(() => {
    return untrack(() => {
      editor?.current.registerPlugin(spellcheckPlugin);

      window.__webview__?.addEventListener('runSpellcheck', async () => {
        const currentEditor = editor?.current;
        if (!currentEditor) return;

        if (!hasSubscription) {
          window.__webview__?.emitEvent('spellcheckResult', {
            success: false,
            needPlanUpgrade: true,
            errors: [],
          });
          return;
        }

        try {
          spellcheckMapping = new Mapping();

          const body = currentEditor.getJSON();
          const errors = await checkSpelling({ body });

          mixpanel.track('spellcheck', { errors: errors.length });

          spellcheckErrors = mapErrors(errors, spellcheckMapping).map((error) => ({
            ...error,
            explanation: decodeHtmlEntities(error.explanation),
          }));

          const { tr } = currentEditor.state;
          tr.setMeta(spellcheckKey, spellcheckErrors);
          tr.setMeta('addToHistory', false);
          currentEditor.view.dispatch(tr);

          window.__webview__?.emitEvent('spellcheckResult', {
            success: true,
            needPlanUpgrade: false,
            errors: spellcheckErrors,
          });
        } catch {
          window.__webview__?.emitEvent('spellcheckResult', {
            success: false,
            needPlanUpgrade: false,
            errors: [],
          });
        }
      });

      window.__webview__?.addEventListener('applySpellCorrection', async (data) => {
        const currentEditor = editor?.current;
        if (!currentEditor) return;

        const { from, to, correction } = data;

        currentEditor.chain().focus().setTextSelection({ from, to }).insertContent(correction).run();

        spellcheckErrors = spellcheckErrors.filter((err) => !(err.from === from && err.to === to));

        if (spellcheckErrors.length === 0) {
          spellcheckMapping = undefined;
        }
      });

      const currentEditor = editor?.current;
      if (currentEditor) {
        const handler = ({ transaction }: { transaction: Transaction }) => {
          handleTransaction({ transaction, editor: currentEditor });
        };

        currentEditor.on('transaction', handler);

        return () => {
          currentEditor.off('transaction', handler);
          editor?.current.unregisterPlugin(spellcheckKey);
        };
      }
    });
  });
</script>
