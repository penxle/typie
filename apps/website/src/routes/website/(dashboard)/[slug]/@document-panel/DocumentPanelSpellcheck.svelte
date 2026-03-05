<script lang="ts">
  import { hide, inline, shift } from '@floating-ui/dom';
  import { createFragment, createMutation } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { createFloatingActions, tooltip } from '@typie/ui/actions';
  import { Icon, RingSpinner } from '@typie/ui/components';
  import { onMount, tick } from 'svelte';
  import ArrowRightIcon from '~icons/lucide/arrow-right';
  import CircleAlertIcon from '~icons/lucide/circle-alert';
  import CircleCheckIcon from '~icons/lucide/circle-check';
  import CopyXIcon from '~icons/lucide/copy-x';
  import XIcon from '~icons/lucide/x';
  import { graphql } from '$mearie';
  import type { Editor } from '$lib/editor/editor.svelte';
  import type { DocumentPanel_Spellcheck_document$key, DocumentPanel_Spellcheck_user$key } from '$mearie';

  type Props = {
    document$key: DocumentPanel_Spellcheck_document$key;
    user$key: DocumentPanel_Spellcheck_user$key;
    editor: Editor;
  };

  let { document$key, user$key, editor }: Props = $props();

  const document = createFragment(
    graphql(`
      fragment DocumentPanel_Spellcheck_document on Document {
        id
      }
    `),
    () => document$key,
  );

  const user = createFragment(
    graphql(`
      fragment DocumentPanel_Spellcheck_user on User {
        id
        subscription {
          id
        }
      }
    `),
    () => user$key,
  );

  let inflight = $state(false);
  let mounted = $state(false);
  let hasChecked = $state(false);
  let checkFailed = $state(false);
  let listContainer = $state<HTMLElement>();

  const activeError = $derived(editor.spellcheckErrors.find((v) => v.active));
  let anchor: ReturnType<typeof createFloatingActions>['anchor'] | undefined = $state();
  let floating: ReturnType<typeof createFloatingActions>['floating'] | undefined = $state();
  let scrollContainer: Element | undefined = $state();

  const [checkSpellingDocument] = createMutation(
    graphql(`
      mutation Editor_Panel_DocumentPanelSpellcheck_CheckSpelling_Mutation($input: CheckSpellingDocumentInput!) {
        checkSpellingDocument(input: $input) {
          id
          nodeId
          startOffset
          endOffset
          context
          corrections
          explanation
        }
      }
    `),
  );

  const runSpellcheck = async () => {
    if (!editor || inflight) {
      return;
    }

    await editor.ready;

    const spellcheckData = editor.getTextWithMappings();
    if (!spellcheckData?.text?.trim()) {
      return;
    }

    inflight = true;
    hasChecked = true;
    checkFailed = false;

    try {
      const resp = await checkSpellingDocument({
        input: {
          documentId: document.data.id,
          text: spellcheckData.text,
          mappings: spellcheckData.mappings,
        },
      });

      editor.spellcheckErrors = resp.checkSpellingDocument.map((error) => ({
        id: error.id,
        context: error.context,
        corrections: [...error.corrections],
        explanation: error.explanation,
        active: false,
      }));

      editor.setTrackedItems(
        0,
        resp.checkSpellingDocument.map((e) => ({
          id: e.id,
          nodeId: e.nodeId,
          startOffset: e.startOffset,
          endOffset: e.endOffset,
        })),
      );
    } catch {
      checkFailed = true;
      editor.setTrackedItems(0, []);
    } finally {
      inflight = false;
    }
  };

  const removeError = (errorId: string) => {
    editor.removeTrackedItems(0, [errorId]);
  };

  const removeSameError = (errorId: string) => {
    const error = editor.spellcheckErrors.find((v) => v.id === errorId);
    if (!error) {
      return;
    }

    const targetIds = editor.spellcheckErrors.filter((v) => v.context === error.context).map((e) => e.id);
    if (targetIds.length > 0) {
      editor.removeTrackedItems(0, targetIds);
    }
  };

  const applyCorrection = (errorId: string, correction: string) => {
    if (!editor) return;

    const item = editor.trackedItems.find((v) => v.group === 0 && v.id === errorId);
    if (!item) return;

    const success = editor.replaceTextInBlock(item.nodeId, item.startOffset, item.endOffset, correction);
    if (success) {
      editor.removeTrackedItems(0, [errorId]);
    }
  };

  const setActiveError = (errorId: string) => {
    for (const error of editor.spellcheckErrors) {
      error.active = error.id === errorId;
    }

    editor.scrollTrackedItemIntoView(errorId);
  };

  const selectErrorRange = (errorId: string) => {
    setActiveError(errorId);

    const item = editor.trackedItems.find((v) => v.group === 0 && v.id === errorId);
    if (!item) return;

    editor.dispatch({
      type: 'setSelection',
      anchorNodeId: item.nodeId,
      anchorOffset: item.startOffset,
      anchorAffinity: 'downstream',
      headNodeId: item.nodeId,
      headOffset: item.endOffset,
      headAffinity: 'upstream',
    });

    editor.focus();
  };

  const handleKeyDown = (e: KeyboardEvent, errorId: string) => {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      setActiveError(errorId);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      const currentIndex = editor.spellcheckErrors.findIndex((v) => v.id === errorId);
      const prevError = editor.spellcheckErrors[currentIndex - 1];
      if (prevError) {
        setActiveError(prevError.id);
        const prevElement = globalThis.document.querySelector(`[data-panel-spellcheck-error="${prevError.id}"]`) as HTMLElement;
        prevElement?.focus();
      }
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      const currentIndex = editor.spellcheckErrors.findIndex((v) => v.id === errorId);
      const nextError = editor.spellcheckErrors[currentIndex + 1];
      if (nextError) {
        setActiveError(nextError.id);
        const nextElement = globalThis.document.querySelector(`[data-panel-spellcheck-error="${nextError.id}"]`) as HTMLElement;
        nextElement?.focus();
      }
    }
  };

  $effect(() => {
    if (editor && !mounted) {
      mounted = true;
    }
  });

  $effect(() => {
    if (mounted && !hasChecked && user.data.subscription) {
      tick().then(() => {
        runSpellcheck();
      });
    }
  });

  $effect(() => {
    if (activeError) {
      const el = listContainer?.querySelector(`[data-panel-spellcheck-error="${activeError.id}"]`) as HTMLElement | null;
      el?.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
    }
  });

  $effect(() => {
    if (activeError && anchor) {
      const overlayElement = globalThis.document.querySelector(`[data-spellcheck-overlay="${activeError.id}"]`) as HTMLElement;

      if (overlayElement) {
        anchor(overlayElement);
      }
    }
  });

  $effect(() => {
    return () => {
      editor?.setTrackedItems(0, []);
    };
  });

  onMount(() => {
    const container = globalThis.document.querySelector('.editor-scroll-container');
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
      맞춤법 검사
      {#if hasChecked && !checkFailed && editor.spellcheckErrors.length > 0}
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
          {editor.spellcheckErrors.length}
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

  {#if inflight}
    <div class={flex({ justifyContent: 'center', alignItems: 'center', paddingY: '40px' })}>
      <RingSpinner style={css.raw({ size: '24px', color: 'text.faint' })} />
    </div>
  {:else if checkFailed}
    <div class={flex({ flexDirection: 'column', alignItems: 'center', gap: '8px', paddingY: '40px' })}>
      <Icon style={css.raw({ color: 'text.faint' })} icon={CircleAlertIcon} size={32} />
      <div class={css({ fontSize: '16px', color: 'text.faint' })}>맞춤법 검사에 실패했습니다</div>
      <div class={css({ fontSize: '14px', color: 'text.faint' })}>잠시 후 다시 시도해주세요</div>
    </div>
  {:else if hasChecked && editor.spellcheckErrors.length === 0}
    <div class={flex({ flexDirection: 'column', alignItems: 'center', gap: '8px', paddingY: '40px' })}>
      <Icon style={css.raw({ color: 'text.faint' })} icon={CircleCheckIcon} size={32} />
      <div class={css({ fontSize: '16px', color: 'text.faint' })}>맞춤법 오류가 없습니다!</div>
    </div>
  {:else if hasChecked}
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
      {#each editor.spellcheckErrors as error, i (i)}
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
          onclick={(e) => {
            if (activeError?.id !== error.id) {
              setActiveError(error.id);
            }
            (e.currentTarget as HTMLElement).focus();
          }}
          onkeydown={(e) => handleKeyDown(e, error.id)}
          role="button"
          tabindex="0"
        >
          <div class={flex({ position: 'absolute', top: '8px', right: '8px', gap: '4px' })}>
            {#if editor.spellcheckErrors.some((e) => e.context === error.context && e.id !== error.id)}
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
                  removeSameError(error.id);
                }}
                type="button"
                use:tooltip={{
                  message: '같은 단어 모두 무시하기',
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
                removeError(error.id);
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
                  whiteSpace: 'pre-line',
                  lineClamp: activeError?.id === error.id ? 'none' : '1',
                })}
              >
                {error.explanation}
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
                  selectErrorRange(error.id);
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

{#if activeError && floating && editor}
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
          if (!editor || !activeError) return;

          applyCorrection(activeError.id, correction);
        }}
        type="button"
      >
        {correction}
        <Icon icon={ArrowRightIcon} size={12} />
      </button>
    {/each}
  </div>
{/if}
