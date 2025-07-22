<script lang="ts">
  import { hide } from '@floating-ui/dom';
  import { Editor, posToDOMRect } from '@tiptap/core';
  import { Plugin, PluginKey, Transaction } from '@tiptap/pm/state';
  import { Mapping } from '@tiptap/pm/transform';
  import { Decoration, DecorationSet } from '@tiptap/pm/view';
  import mixpanel from 'mixpanel-browser';
  import { nanoid } from 'nanoid';
  import { untrack } from 'svelte';
  import ArrowRightIcon from '~icons/lucide/arrow-right';
  import CircleHelpIcon from '~icons/lucide/circle-help';
  import SpellCheckIcon from '~icons/lucide/spell-check';
  import { graphql } from '$graphql';
  import { createFloatingActions } from '$lib/actions';
  import { Icon, RingSpinner, Tooltip } from '$lib/components';
  import { css } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';
  import PlanUpgradeModal from '../PlanUpgradeModal.svelte';
  import ToolbarButton from './ToolbarButton.svelte';
  import type { VirtualElement } from '@floating-ui/dom';
  import type { Ref } from '$lib/utils';

  type Props = {
    editor?: Ref<Editor>;
    subscription: boolean;
  };

  type SpellingError = {
    id: string;
    from: number;
    to: number;
    context: string;
    corrections: string[];
    explanation: string;
  };

  let { editor, subscription }: Props = $props();

  const key = new PluginKey<DecorationSet>('spellcheck');

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

      const map = (pos: number) => {
        // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
        const result = mapping!.mapResult(pos);
        if (result.deleted) {
          return null;
        }

        return result.pos;
      };

      errors = resp
        .map((error) => ({
          id: nanoid(),
          from: map(error.from),
          to: map(error.to),
          context: error.context,
          corrections: error.corrections,
          explanation: error.explanation,
        }))
        .filter((error): error is SpellingError => error.from !== null && error.to !== null);

      mapping = undefined;

      const { tr } = editor.current.view.state;
      tr.setMeta(key, errors);
      tr.setMeta('addToHistory', false);
      editor.current.view.dispatch(tr);
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
        editor?.current.registerPlugin(
          new Plugin({
            key,
            state: {
              init: () => DecorationSet.empty,
              apply: (tr, state, _, newState) => {
                const meta = tr.getMeta(key) as { from: number; to: number }[];
                if (meta) {
                  const decorations: Decoration[] = [];

                  for (const error of meta) {
                    const decoration = Decoration.inline(error.from, error.to, {
                      class: css({
                        textDecoration: 'underline',
                        textDecorationColor: 'text.danger',
                        textDecorationStyle: 'wavy',
                        textUnderlineOffset: '2px',
                      }),
                    });

                    decorations.push(decoration);
                  }

                  return DecorationSet.create(newState.doc, decorations);
                }

                if (tr.docChanged) {
                  return state.map(tr.mapping, tr.doc);
                }

                return state;
              },
            },
            props: {
              decorations: (state) => key.getState(state),
            },
          }),
        );
      });
    }

    return () => {
      editor?.current.unregisterPlugin(key);
    };
  });

  const handleTransaction = ({ transaction }: { transaction: Transaction }) => {
    if (transaction.docChanged) {
      mapping?.appendMapping(transaction.mapping);

      const newErrors: SpellingError[] = [];

      for (const error of errors) {
        const { from, to } = error;

        const map = (pos: number) => {
          const result = transaction.mapping.mapResult(pos);
          if (result.deleted) {
            return null;
          }

          return result.pos;
        };

        const mappedFrom = map(from);
        const mappedTo = map(to);

        if (mappedFrom !== null && mappedTo !== null) {
          newErrors.push({
            ...error,
            from: mappedFrom,
            to: mappedTo,
          });
        }
      }

      errors = newErrors;
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
