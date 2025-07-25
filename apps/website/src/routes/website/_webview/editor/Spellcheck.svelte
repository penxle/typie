<script lang="ts">
  import { Mapping } from '@tiptap/pm/transform';
  import mixpanel from 'mixpanel-browser';
  import { onMount } from 'svelte';
  import { graphql } from '$graphql';
  import { createSpellcheckPlugin, decodeHtmlEntities, mapErrors, spellcheckKey, updateErrorPositions } from '$lib/editor/spellcheck';
  import type { Editor } from '@tiptap/core';
  import type { Transaction } from '@tiptap/pm/state';
  import type { SpellingError } from '$lib/editor/spellcheck';
  import type { Ref } from '$lib/utils';

  type Props = {
    editor?: Ref<Editor>;
  };

  let { editor }: Props = $props();

  let spellcheckErrors = $state<SpellingError[]>([]);
  let spellcheckMapping = $state<Mapping>();

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

  onMount(() => {
    if (!editor?.current) return;

    editor.current.registerPlugin(spellcheckPlugin);

    window.__webview__?.setProcedure('checkSpelling', async () => {
      if (!editor?.current) return;

      spellcheckMapping = new Mapping();

      const body = editor.current.getJSON();
      const errors = await checkSpelling({ body });

      mixpanel.track('spellcheck', { errors: errors.length });

      spellcheckErrors = mapErrors(errors, spellcheckMapping).map((error) => ({
        ...error,
        explanation: decodeHtmlEntities(error.explanation),
      }));

      const { tr } = editor.current.state;
      tr.setMeta(spellcheckKey, spellcheckErrors);
      tr.setMeta('addToHistory', false);
      editor.current.view.dispatch(tr);

      return {
        errors: spellcheckErrors,
      };
    });

    window.__webview__?.addEventListener('applySpellCorrection', async (data) => {
      if (!editor?.current) return;

      const { from, to, correction } = data;

      editor.current.chain().focus().setTextSelection({ from, to }).insertContent(correction).run();

      spellcheckErrors = spellcheckErrors.filter((err) => !(err.from === from && err.to === to));
      if (spellcheckErrors.length === 0) {
        spellcheckMapping = undefined;
      }
    });

    const handler = ({ transaction }: { transaction: Transaction }) => {
      handleTransaction({ transaction, editor: editor.current });
    };

    editor.current.on('transaction', handler);

    return () => {
      editor?.current.off('transaction', handler);
      editor?.current.unregisterPlugin(spellcheckKey);
    };
  });
</script>
