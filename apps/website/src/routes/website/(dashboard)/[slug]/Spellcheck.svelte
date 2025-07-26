<script lang="ts">
  import { hide } from '@floating-ui/dom';
  import { Editor, posToDOMRect } from '@tiptap/core';
  import { Plugin, PluginKey, Transaction } from '@tiptap/pm/state';
  import { Decoration, DecorationSet } from '@tiptap/pm/view';
  import mixpanel from 'mixpanel-browser';
  import { nanoid } from 'nanoid';
  import { untrack } from 'svelte';
  import { absolutePositionToRelativePosition, relativePositionToAbsolutePosition, ySyncPluginKey } from 'y-prosemirror';
  import ArrowRightIcon from '~icons/lucide/arrow-right';
  import CircleHelpIcon from '~icons/lucide/circle-help';
  import SpellCheckIcon from '~icons/lucide/spell-check';
  import { graphql } from '$graphql';
  import { createFloatingActions } from '$lib/actions';
  import { Icon, RingSpinner, Tooltip } from '$lib/components';
  import { Toast } from '$lib/notification';
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

  let { editor, subscription }: Props = $props();

  let inflight = $state(false);
  let mounted = $state(false);

  let planUpgradeOpen = $state(false);

  let errors = $state<SpellcheckError[]>([]);
  let activeError = $state<SpellcheckError>();

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
    if (!subscription) {
      planUpgradeOpen = true;
      return;
    }

    if (!editor?.current || inflight) {
      return;
    }

    inflight = true;

    try {
      const body = editor.current.getJSON();
      const resp = await checkSpelling({ body });

      const { binding } = ySyncPluginKey.getState(editor.current.view.state);
      errors = resp.map((error) => ({
        id: nanoid(),
        ...error,
        relativeFrom: absolutePositionToRelativePosition(error.from, binding.type, binding.mapping),
        relativeTo: absolutePositionToRelativePosition(error.to, binding.type, binding.mapping),
      }));

      mixpanel.track('spellcheck', { errors: errors.length });
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

    if (transaction.selectionSet) {
      activeError = errors.find((error) => {
        return error.from <= transaction.selection.from && error.to >= transaction.selection.to;
      });
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
    if (activeError) {
      const element: VirtualElement = {
        getBoundingClientRect: () => {
          // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
          return posToDOMRect(editor!.current.view, activeError!.from, activeError!.to);
        },
      };

      anchor(element);
    }
  });
</script>

<PlanUpgradeModal bind:open={planUpgradeOpen} />

{#if inflight}
  <div class={center({ size: '48px' })}>
    <RingSpinner style={css.raw({ size: '24px', color: 'text.faint' })} />
  </div>
{:else}
  <ToolbarButton icon={SpellCheckIcon} label="맞춤법" onclick={spellcheck} size="large" />
{/if}

{#if activeError}
  <div class={flex({ alignItems: 'center', gap: '4px', backgroundColor: 'surface.default', zIndex: '10' })} use:floating>
    {#each activeError.corrections as correction, index (index)}
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
          // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
          const { id, from, to } = activeError!;
          editor?.current.chain().setTextSelection({ from, to }).insertContent(correction).focus().run();
          errors = errors.filter((error) => error.id !== id);
          activeError = undefined;
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
        {@html activeError?.explanation}
      {/snippet}

      <Icon style={css.raw({ color: 'text.danger' })} icon={CircleHelpIcon} size={16} />
    </Tooltip>
  </div>
{/if}
