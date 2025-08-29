<script lang="ts">
  import { hide, inline, shift } from '@floating-ui/dom';
  import { Plugin, PluginKey, Transaction } from '@tiptap/pm/state';
  import { Decoration, DecorationSet } from '@tiptap/pm/view';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { createFloatingActions, tooltip } from '@typie/ui/actions';
  import { HorizontalDivider, Icon, RingSpinner } from '@typie/ui/components';
  import mixpanel from 'mixpanel-browser';
  import { nanoid } from 'nanoid';
  import { onMount, tick, untrack } from 'svelte';
  import { absolutePositionToRelativePosition, relativePositionToAbsolutePosition, ySyncPluginKey } from 'y-prosemirror';
  import ArrowRightIcon from '~icons/lucide/arrow-right';
  import CircleAlertIcon from '~icons/lucide/circle-alert';
  import CircleCheckIcon from '~icons/lucide/circle-check';
  import CopyXIcon from '~icons/lucide/copy-x';
  import XIcon from '~icons/lucide/x';
  import { fragment, graphql } from '$graphql';
  import type { Editor } from '@tiptap/core';
  import type { Ref } from '@typie/ui/utils';
  import type { Editor_Panel_PanelSpellcheck_user } from '$graphql';

  type Props = {
    $user: Editor_Panel_PanelSpellcheck_user;
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

  let { $user: _user, editor }: Props = $props();

  const user = fragment(
    _user,
    graphql(`
      fragment Editor_Panel_PanelSpellcheck_user on User {
        id

        subscription {
          id
        }
      }
    `),
  );

  let inflight = $state(false);
  let mounted = $state(false);
  let errors = $state<SpellcheckError[]>([]);
  let activeError = $state<SpellcheckError>();
  let hasChecked = $state(false);
  let checkFailed = $state(false);
  let anchor: ReturnType<typeof createFloatingActions>['anchor'] | undefined = $state();
  let floating: ReturnType<typeof createFloatingActions>['floating'] | undefined = $state();

  let scrollContainer: Element | undefined = $state();

  const checkSpelling = graphql(`
    mutation Editor_Panel_Spellcheck_CheckSpelling_Mutation($input: CheckSpellingInput!) {
      checkSpelling(input: $input) {
        from
        to
        context
        corrections
        explanation
      }
    }
  `);

  const runSpellcheck = async () => {
    if (!editor?.current || inflight) {
      return;
    }

    inflight = true;
    hasChecked = true;
    checkFailed = false;

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

      mixpanel.track('spellcheck', { errors: errors.length, via: 'panel' });
    } catch {
      checkFailed = true;
      errors = [];
    } finally {
      inflight = false;
    }
  };

  const applyCorrection = (errorId: string, correction: string) => {
    if (!editor?.current) return;

    const error = errors.find((e) => e.id === errorId);
    if (!error) return;

    editor.current.chain().setTextSelection({ from: error.from, to: error.to }).insertContent(correction).run();
    errors = errors.filter((e) => e.id !== errorId);
  };

  const scrollToError = (error: SpellcheckError) => {
    if (!editor?.current) return;

    editor.current
      .chain()
      .setTextSelection({ from: error.to, to: error.to })
      .scrollIntoViewFixed({ pos: error.from, position: editor.current.storage.typewriter.position ?? 0.5, animate: true })
      .run();

    activeError = error;
  };

  const selectErrorRange = (error: SpellcheckError) => {
    if (!editor?.current) return;

    editor.current.chain().focus().setTextSelection({ from: error.from, to: error.to }).run();
  };

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

    if (transaction.selectionSet) {
      const newActiveError = errors.find((error) => {
        return error.from <= transaction.selection.from && error.to >= transaction.selection.to;
      });

      if (newActiveError && newActiveError !== activeError) {
        activeError = newActiveError;
        setTimeout(() => {
          const errorElement = document.querySelector(`[data-panel-spellcheck-error="${newActiveError.id}"]`);
          if (errorElement) {
            errorElement.scrollIntoView({ behavior: 'smooth', block: 'center' });
          }
        }, 0);
      } else if (!newActiveError) {
        activeError = undefined;
      }
    }
  };

  $effect(() => {
    if (editor?.current && !mounted) {
      mounted = true;
    }
  });

  $effect(() => {
    void errors;
    untrack(() => {
      if (editor?.current) {
        editor.current.view.dispatch(editor.current.view.state.tr);
      }
    });
  });

  $effect(() => {
    if (mounted && hasChecked) {
      return untrack(() => {
        const key = new PluginKey('spellcheck-panel');

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
                      'data-spellcheck-error': error.id,
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
    if (mounted && !hasChecked && $user.subscription) {
      // NOTE: tick 후 하지 않으면 빈 문서로 검사하는 문제가 있음
      tick().then(() => {
        runSpellcheck();
      });
    }
  });

  $effect(() => {
    if (activeError && editor?.current) {
      const element = editor.current.view.dom.querySelector(`[data-spellcheck-error="${activeError.id}"]`);

      if (element instanceof HTMLElement && anchor) {
        anchor(element);
      }
    }
  });

  onMount(() => {
    const container = document.querySelector('.editor-scroll-container');
    if (!container) return;

    ({ anchor, floating } = createFloatingActions({
      placement: 'top',
      offset: 4,
      middleware: [
        inline(),
        hide({
          strategy: 'escaped',
          boundary: container,
          padding: 32,
        }),
        shift({ padding: 8 }),
      ],
    }));

    scrollContainer = container;

    return () => {
      anchor = undefined;
      floating = undefined;
      scrollContainer = undefined;
    };
  });
</script>

<div
  class={flex({
    flexDirection: 'column',
    paddingTop: '16px',
    minWidth: 'var(--min-width)',
    width: 'var(--width)',
    maxWidth: 'var(--max-width)',
    height: 'full',
  })}
>
  <div class={flex({ flexDirection: 'column', gap: '6px', paddingX: '20px' })}>
    <div class={flex({ justifyContent: 'space-between', alignItems: 'center' })}>
      <div class={flex({ alignItems: 'center', gap: '6px' })}>
        <div class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'text.subtle' })}>맞춤법 검사</div>
        {#if hasChecked && !checkFailed && errors.length > 0}
          <div
            class={css({
              borderRadius: '4px',
              paddingX: '6px',
              paddingY: '2px',
              fontSize: '11px',
              fontWeight: 'semibold',
              color: 'text.danger',
              backgroundColor: 'accent.danger.subtle',
            })}
          >
            {errors.length}
          </div>
        {/if}
      </div>

      {#if !inflight && hasChecked}
        <button
          class={css({
            fontSize: '13px',
            fontWeight: 'medium',
            color: 'text.faint',
            transition: 'common',
            _hover: { color: 'text.subtle' },
          })}
          onclick={runSpellcheck}
          type="button"
        >
          다시 검사
        </button>
      {/if}
    </div>
  </div>

  <HorizontalDivider style={css.raw({ marginTop: '16px' })} color="secondary" />

  {#if inflight}
    <div class={flex({ justifyContent: 'center', alignItems: 'center', paddingY: '40px' })}>
      <RingSpinner style={css.raw({ size: '24px', color: 'text.faint' })} />
    </div>
  {:else if (hasChecked && checkFailed) || !$user.subscription}
    <div class={flex({ flexDirection: 'column', alignItems: 'center', gap: '8px', paddingY: '40px' })}>
      <Icon style={css.raw({ color: 'text.faint' })} icon={CircleAlertIcon} size={32} />
      <div class={css({ fontSize: '16px', color: 'text.faint' })}>맞춤법 검사에 실패했습니다</div>
      <div class={css({ fontSize: '14px', color: 'text.faint' })}>잠시 후 다시 시도해주세요</div>
    </div>
  {:else if hasChecked && errors.length === 0}
    <div class={flex({ flexDirection: 'column', alignItems: 'center', gap: '8px', paddingY: '40px' })}>
      <Icon style={css.raw({ color: 'text.faint' })} icon={CircleCheckIcon} size={32} />
      <div class={css({ fontSize: '16px', color: 'text.faint' })}>맞춤법 오류가 없습니다!</div>
    </div>
  {:else if hasChecked}
    <div
      class={flex({
        flexDirection: 'column',
        gap: '12px',
        paddingX: '12px',
        paddingTop: '16px',
        paddingBottom: '100px',
        overflowY: 'auto',
      })}
    >
      {#each errors as error (error.id)}
        <div
          class={css({
            position: 'relative',
            borderWidth: '1px',
            borderColor: activeError?.id === error.id ? 'border.danger!' : 'border.default',
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
          data-panel-spellcheck-error={error.id}
          onclick={() => {
            if (activeError?.id === error.id) {
              activeError = undefined;
            } else {
              scrollToError(error);
            }
          }}
          onkeydown={(e) => {
            if (e.key === 'Enter' || e.key === ' ') {
              e.preventDefault();
              if (activeError?.id === error.id) {
                activeError = undefined;
              } else {
                scrollToError(error);
              }
            } else if (e.key === 'ArrowUp') {
              e.preventDefault();
              const currentIndex = errors.findIndex((err) => err.id === error.id);
              const prevError = errors[currentIndex - 1];
              if (prevError) {
                scrollToError(prevError);
                const prevElement = document.querySelector(`[data-panel-spellcheck-error="${prevError.id}"]`) as HTMLElement;
                prevElement?.focus();
              }
            } else if (e.key === 'ArrowDown') {
              e.preventDefault();
              const currentIndex = errors.findIndex((err) => err.id === error.id);
              const nextError = errors[currentIndex + 1];
              if (nextError) {
                scrollToError(nextError);
                const nextElement = document.querySelector(`[data-panel-spellcheck-error="${nextError.id}"]`) as HTMLElement;
                nextElement?.focus();
              }
            }
          }}
          role="button"
          tabindex="0"
        >
          <div class={flex({ position: 'absolute', top: '8px', right: '8px', gap: '4px' })}>
            {#if errors.some((e) => e.context === error.context && e.id !== error.id)}
              <button
                class={css({
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
                  const contextToRemove = error.context;
                  const errorsToRemove = errors.filter((e) => e.context === contextToRemove);
                  const currentActiveError = activeError;
                  errors = errors.filter((e) => e.context !== contextToRemove);

                  if (currentActiveError && errorsToRemove.some((e) => e.id === currentActiveError.id) && editor?.current) {
                    activeError = undefined;
                  }
                }}
                type="button"
                use:tooltip={{
                  message: '같은 단어 모두 무시',
                  placement: 'top',
                }}
              >
                <Icon icon={CopyXIcon} size={14} />
              </button>
            {/if}
            <button
              class={css({
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
                errors = errors.filter((e) => e.id !== error.id);
                if (activeError?.id === error.id && editor?.current) {
                  activeError = undefined;
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
          </div>
          <div class={flex({ flexDirection: 'column', gap: '8px' })}>
            <div class={css({ fontSize: '14px', color: 'text.default' })}>
              {error.context}
            </div>

            {#if error.explanation}
              <div
                class={css({
                  fontSize: '12px',
                  color: 'text.faint',
                  lineClamp: activeError?.id === error.id ? 'none' : '1',
                })}
              >
                <!-- eslint-disable-next-line svelte/no-at-html-tags -->
                {@html error.explanation}
              </div>
            {/if}

            <div class={flex({ flexWrap: 'wrap', gap: '8px' })}>
              {#if error.corrections.length > 0}
                {#each error.corrections as correction (correction)}
                  <button
                    class={css({
                      borderWidth: '1px',
                      borderColor: 'border.danger',
                      borderRadius: '4px',
                      paddingX: '8px',
                      paddingY: '4px',
                      fontSize: '13px',
                      fontWeight: 'semibold',
                      color: 'text.danger',
                      backgroundColor: 'accent.danger.subtle',
                      transition: 'common',
                      _hover: {
                        backgroundColor: { base: 'red.100', _dark: 'dark.red.800' },
                      },
                      _focusVisible: {
                        backgroundColor: { base: 'red.100', _dark: 'dark.red.800' },
                      },
                    })}
                    onclick={(e) => {
                      e.stopPropagation();
                      applyCorrection(error.id, correction);
                    }}
                    type="button"
                  >
                    {correction}
                  </button>
                {/each}
              {/if}
              <button
                class={css({
                  borderWidth: '1px',
                  borderColor: 'border.default',
                  borderRadius: '4px',
                  paddingX: '8px',
                  paddingY: '4px',
                  fontSize: '13px',
                  fontWeight: 'semibold',
                  transition: 'common',
                  backgroundColor: 'surface.default',
                  _hover: {
                    backgroundColor: 'surface.muted',
                  },
                  _focusVisible: {
                    backgroundColor: 'surface.muted',
                  },
                })}
                onclick={(e) => {
                  e.stopPropagation();
                  selectErrorRange(error);
                }}
                type="button"
              >
                직접 수정
              </button>
            </div>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

{#if activeError && floating && editor?.current}
  <div class={flex({ alignItems: 'center', gap: '4px', zIndex: 'overEditor', wrap: 'wrap' })} use:floating={{ appendTo: scrollContainer }}>
    {#each activeError.corrections as correction (correction)}
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
          if (!editor?.current || !activeError) return;

          const { id, from, to } = activeError;
          editor.current.chain().setTextSelection({ from, to }).insertContent(correction).run();
          errors = errors.filter((error) => error.id !== id);
          activeError = undefined;
        }}
        type="button"
      >
        {correction}
        <Icon icon={ArrowRightIcon} size={12} />
      </button>
    {/each}
  </div>
{/if}
