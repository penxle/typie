<script lang="ts">
  import { hide } from '@floating-ui/dom';
  import { Editor, posToDOMRect } from '@tiptap/core';
  import { Mapping } from '@tiptap/pm/transform';
  import mixpanel from 'mixpanel-browser';
  import { untrack } from 'svelte';
  import ArrowRightIcon from '~icons/lucide/arrow-right';
  import CircleHelpIcon from '~icons/lucide/circle-help';
  import SpellCheckIcon from '~icons/lucide/spell-check';
  import { graphql } from '$graphql';
  import { createFloatingActions } from '$lib/actions';
  import { Icon, RingSpinner, Tooltip } from '$lib/components';
  import { createSpellcheckPlugin, mapErrors, spellcheckKey, updateErrorPositions } from '$lib/editor/spellcheck';
  import { Toast } from '$lib/notification';
  import { css } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';
  import PlanUpgradeModal from '../PlanUpgradeModal.svelte';
  import ToolbarButton from './ToolbarButton.svelte';
  import type { VirtualElement } from '@floating-ui/dom';
  import type { Transaction } from '@tiptap/pm/state';
  import type { SpellingError } from '$lib/editor/spellcheck';
  import type { Ref } from '$lib/utils';

  type Props = {
    editor?: Ref<Editor>;
    subscription: boolean;
  };

  let { editor, subscription }: Props = $props();

  const key = spellcheckKey;

  let inflight = $state(false);
  let mounted = $state(false);

  let planUpgradeOpen = $state(false);

  let errors = $state<SpellingError[]>([]);
  let activeId = $state<string>();

  let mapping = $state<Mapping>();

  const checkSpelling = graphql(`
    mutation Editor_Spellcheck_CheckSpelling_Mutation($input: CheckSpellingInput!) {
      checkSpelling(input: $input) {
        from
        to
        context
        corrections
        explanation
      }
    }
  `);

  const { anchor, floating } = createFloatingActions({
    placement: 'top',
    offset: 4,
    middleware: [
      hide({
        strategy: 'escaped',
        // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
        boundary: document.querySelector('.editor')!,
        padding: 32,
      }),
    ],
  });

  const spellcheck = async () => {
    if (!editor?.current || inflight) {
      return;
    }

    inflight = true;

    try {
      mapping = new Mapping();

      const body = editor.current.getJSON();
      const resp = await checkSpelling({ body });

      mixpanel.track('spellcheck', { errors: resp.length });

      errors = mapErrors(resp, mapping);

      mapping = undefined;

      const { tr } = editor.current.view.state;
      tr.setMeta(key, errors);
      tr.setMeta('addToHistory', false);
      editor.current.view.dispatch(tr);
    } catch {
      Toast.error('맞춤법 검사에 실패했습니다');
    } finally {
      inflight = false;
    }
  };

  $effect(() => {
    if (editor?.current && !mounted) {
      mounted = true;
    }
  });

  $effect(() => {
    if (mounted) {
      untrack(() => {
        editor?.current.registerPlugin(createSpellcheckPlugin(spellcheckKey));
      });
    }

    return () => {
      editor?.current.unregisterPlugin(spellcheckKey);
    };
  });

  const handleTransaction = ({ transaction }: { transaction: Transaction }) => {
    if (transaction.docChanged) {
      mapping?.appendMapping(transaction.mapping);
      errors = updateErrorPositions(errors, transaction);
    }
  };

  const handleSelectionUpdate = ({ editor }: { editor: Editor }) => {
    const { from, to } = editor.view.state.selection;
    activeId = errors.find((error) => from >= error.from && to <= error.to)?.id;
  };

  $effect(() => {
    if (mounted) {
      editor?.current.on('selectionUpdate', handleSelectionUpdate);
      editor?.current.on('transaction', handleTransaction);

      return () => {
        editor?.current.off('selectionUpdate', handleSelectionUpdate);
        editor?.current.off('transaction', handleTransaction);
      };
    }
  });

  $effect(() => {
    if (activeId) {
      const element: VirtualElement = {
        getBoundingClientRect: () => {
          const error = errors.find((error) => error.id === activeId);
          // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
          return posToDOMRect(editor!.current.view, error!.from, error!.to);
        },
      };

      anchor(element);
    }
  });
</script>

<PlanUpgradeModal bind:open={planUpgradeOpen} />

{#if subscription}
  {#if inflight}
    <div class={center({ size: '48px' })}>
      <RingSpinner style={css.raw({ size: '24px', color: 'text.faint' })} />
    </div>
  {:else}
    <ToolbarButton icon={SpellCheckIcon} label="맞춤법" onclick={spellcheck} size="large" />
  {/if}
{:else}
  <ToolbarButton icon={SpellCheckIcon} label="맞춤법" onclick={() => (planUpgradeOpen = true)} size="large" />
{/if}

{#if activeId}
  {@const error = errors.find((error) => error.id === activeId)}

  {#if error}
    <div class={flex({ alignItems: 'center', gap: '4px', backgroundColor: 'surface.default', zIndex: '10' })} use:floating>
      {#each error.corrections as correction, index (index)}
        <button
          class={flex({
            justifyContent: 'space-between',
            alignItems: 'center',
            gap: '4px',
            borderWidth: '1px',
            borderColor: 'border.danger',
            borderRadius: '4px',
            paddingX: '4px',
            paddingY: '4px',
            fontSize: '13px',
            fontWeight: 'semibold',
            color: 'text.danger',
            backgroundColor: 'accent.danger.subtle',
            transition: 'common',
            boxShadow: 'small',
            _hover: {
              backgroundColor: { base: 'red.100', _dark: 'dark.red.800' },
            },
          })}
          onclick={() => {
            const { from, to } = error;

            errors = errors.filter((error) => error.id !== activeId);
            activeId = undefined;

            editor?.current.chain().setTextSelection({ from, to }).insertContent(correction).focus().run();
            editor?.current.commands.command(({ tr }) => {
              tr.setMeta(key, errors);
              tr.setMeta('addToHistory', false);

              return true;
            });
          }}
          type="button"
        >
          {correction}

          <Icon icon={ArrowRightIcon} size={12} />
        </button>
      {/each}

      <Tooltip placement="right" tooltipStyle={css.raw({ maxWidth: '200px' })}>
        {#snippet message()}
          <!-- eslint-disable-next-line svelte/no-at-html-tags -->
          {@html error.explanation}
        {/snippet}

        <Icon style={css.raw({ color: 'text.danger' })} icon={CircleHelpIcon} size={16} />
      </Tooltip>
    </div>
  {/if}
{/if}
