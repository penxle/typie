<script lang="ts">
  import { createFragment, createMutation } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { Button, Icon, RingSpinner } from '@typie/ui/components';
  import { Toast } from '@typie/ui/notification';
  import { onMount } from 'svelte';
  import CircleAlertIcon from '~icons/lucide/circle-alert';
  import CircleCheckIcon from '~icons/lucide/circle-check';
  import CopyXIcon from '~icons/lucide/copy-x';
  import SpellCheckIcon from '~icons/lucide/spell-check';
  import XIcon from '~icons/lucide/x';
  import { graphql } from '$mearie';
  import type { Editor } from '$lib/editor-ffi/editor.svelte';
  import type { DocumentPanelV2_Spellcheck_document$key, DocumentPanelV2_Spellcheck_user$key } from '$mearie';

  type Props = {
    document$key: DocumentPanelV2_Spellcheck_document$key;
    user$key: DocumentPanelV2_Spellcheck_user$key;
    editor: Editor | undefined;
  };

  let { document$key, user$key, editor }: Props = $props();

  const document = createFragment(
    graphql(`
      fragment DocumentPanelV2_Spellcheck_document on Document {
        id
      }
    `),
    () => document$key,
  );

  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  const _user = createFragment(
    graphql(`
      fragment DocumentPanelV2_Spellcheck_user on User {
        id
        subscription {
          id
        }
      }
    `),
    () => user$key,
  );

  const [checkSpellingDocumentV2] = createMutation(
    graphql(`
      mutation Editor_Panel_DocumentPanelV2Spellcheck_CheckSpellingV2_Mutation($input: CheckSpellingDocumentV2Input!) {
        checkSpellingDocumentV2(input: $input) {
          id
          start
          end
          context
          corrections
          explanation
        }
      }
    `),
  );

  let inflight = $state(false);
  let hasChecked = $state(false);
  let checkFailed = $state(false);
  let abortController: AbortController | undefined;
  let listContainer = $state<HTMLElement>();

  const activeError = $derived(
    editor && editor.activeSpellcheckErrorId ? editor.spellcheckErrors.find((e) => e.id === editor.activeSpellcheckErrorId) : undefined,
  );

  const runSpellcheck = async () => {
    if (!editor || inflight) return;

    abortController?.abort();
    const controller = new AbortController();
    abortController = controller;
    inflight = true;
    hasChecked = true;
    checkFailed = false;

    editor.clearSpellcheckErrors();
    editor.installSpellcheckDecorations();

    const text = editor.proseText();
    if (!text.trim()) {
      inflight = false;
      return;
    }

    try {
      const resp = await checkSpellingDocumentV2(
        {
          input: { documentId: document.data.id, text },
        },
        { signal: controller.signal },
      );
      if (abortController !== controller || controller.signal.aborted) return;
      if (editor.proseText() !== text) {
        cancelCheckForDocumentEdit();
        return;
      }

      const items = resp.checkSpellingDocumentV2
        .map((err) => {
          const sel = editor.proseToSelection(err.start, err.end);
          if (!sel) return null;
          return {
            id: err.id,
            selection: sel,
            context: err.context,
            corrections: [...err.corrections],
            explanation: err.explanation,
          };
        })
        .filter((x): x is NonNullable<typeof x> => x !== null);

      editor.setSpellcheckErrors(items);
    } catch (err) {
      if (abortController === controller && (!(err instanceof Error) || err.name !== 'AbortError')) {
        checkFailed = true;
      }
    } finally {
      if (abortController === controller) {
        abortController = undefined;
        inflight = false;
      }
    }
  };

  const cancelCheckForDocumentEdit = () => {
    if (!inflight) return;
    abortController?.abort();
    abortController = undefined;
    inflight = false;
    hasChecked = false;
    checkFailed = false;
    editor?.clearSpellcheckErrors();
    Toast.success('내용이 수정되어 맞춤법 검사가 취소됐어요.');
  };

  const applyCorrection = (errorId: string, correction: string) => {
    if (!editor) return;
    if (editor.readOnly) {
      Toast.error('잠긴 문서는 편집할 수 없어요.');
      editor.focus();
      return;
    }
    editor.applySpellcheckCorrection(errorId, correction);
    editor.focus();
  };

  const removeError = (errorId: string) => {
    if (!editor) return;
    editor.removeSpellcheckError(errorId);
    editor.focus();
  };

  const removeSameError = (errorId: string) => {
    if (!editor) return;
    const err = editor.spellcheckErrors.find((e) => e.id === errorId);
    if (!err) return;
    editor.removeSpellcheckErrorsByContext(err.context);
    editor.focus();
  };

  const setActiveError = (errorId: string) => {
    if (!editor) return;
    editor.setActiveSpellcheckError(errorId);
  };

  const selectErrorRange = (errorId: string) => {
    if (!editor) return;
    setActiveError(errorId);

    const range = editor.trackedRanges.find((r) => r.id === errorId);
    if (!range) return;

    editor.enqueue({
      type: 'selection',
      op: {
        type: 'set',
        selection: { anchor: range.anchor, head: range.head },
      },
    });
    editor.focus();
  };

  const handleKeyDown = (e: KeyboardEvent, errorId: string) => {
    if (!editor) return;
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      setActiveError(errorId);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      const idx = editor.spellcheckErrors.findIndex((v) => v.id === errorId);
      const prev = editor.spellcheckErrors[idx - 1];
      if (prev) {
        setActiveError(prev.id);
        const el = globalThis.document.querySelector(`[data-panel-spellcheck-error="${prev.id}"]`) as HTMLElement | null;
        el?.focus();
      }
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      const idx = editor.spellcheckErrors.findIndex((v) => v.id === errorId);
      const next = editor.spellcheckErrors[idx + 1];
      if (next) {
        setActiveError(next.id);
        const el = globalThis.document.querySelector(`[data-panel-spellcheck-error="${next.id}"]`) as HTMLElement | null;
        el?.focus();
      }
    }
  };

  $effect(() => {
    if (activeError) {
      const el = listContainer?.querySelector(`[data-panel-spellcheck-error="${activeError.id}"]`) as HTMLElement | null;
      el?.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
    }
  });

  $effect(() => {
    const activeEditor = editor;
    if (!activeEditor) return;

    return activeEditor.on('state_changed', (_, { fields }) => {
      if (fields.includes('doc')) {
        cancelCheckForDocumentEdit();
      }
    });
  });

  onMount(() => {
    return () => {
      abortController?.abort();
      editor?.clearSpellcheckErrors();
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
      {#if editor && hasChecked && !checkFailed && editor.spellcheckErrors.length > 0}
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

  {#if !hasChecked && !inflight}
    <div
      class={flex({
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: 'center',
        gap: '20px',
        paddingY: '60px',
      })}
    >
      <div
        class={center({
          size: '64px',
          borderRadius: '16px',
          backgroundColor: 'surface.muted',
          color: 'text.faint',
        })}
      >
        <Icon icon={SpellCheckIcon} size={28} />
      </div>

      <div class={flex({ flexDirection: 'column', alignItems: 'center', gap: '8px' })}>
        <p class={css({ fontSize: '13px', color: 'text.faint', textAlign: 'center' })}>
          글의 맞춤법과 띄어쓰기를
          <br />
          검사해보세요
        </p>
      </div>

      <Button onclick={runSpellcheck} size="sm" variant="secondary">검사 시작</Button>
    </div>
  {:else if inflight}
    <div class={flex({ justifyContent: 'center', alignItems: 'center', paddingY: '40px' })}>
      <RingSpinner style={css.raw({ size: '24px', color: 'text.faint' })} />
    </div>
  {:else if checkFailed}
    <div class={flex({ flexDirection: 'column', alignItems: 'center', gap: '8px', paddingY: '40px' })}>
      <Icon style={css.raw({ color: 'text.faint' })} icon={CircleAlertIcon} size={32} />
      <div class={css({ fontSize: '16px', color: 'text.faint' })}>맞춤법 검사에 실패했습니다</div>
      <div class={css({ fontSize: '14px', color: 'text.faint' })}>잠시 후 다시 시도해주세요</div>
    </div>
  {:else if editor && editor.spellcheckErrors.length === 0}
    <div class={flex({ flexDirection: 'column', alignItems: 'center', gap: '8px', paddingY: '40px' })}>
      <Icon style={css.raw({ color: 'text.faint' })} icon={CircleCheckIcon} size={32} />
      <div class={css({ fontSize: '16px', color: 'text.faint' })}>맞춤법 오류가 없습니다!</div>
    </div>
  {:else if editor}
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
            _hover: { borderColor: 'border.strong', backgroundColor: 'surface.subtle' },
            _focusVisible: { borderColor: 'border.strong', backgroundColor: 'surface.subtle' },
          })}
          data-panel-spellcheck-error={error.id}
          onclick={(e) => {
            if (activeError?.id !== error.id) setActiveError(error.id);
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
                  _hover: { backgroundColor: 'interactive.hover', color: 'text.subtle' },
                  _focusVisible: { backgroundColor: 'interactive.hover', color: 'text.subtle' },
                })}
                onclick={(e) => {
                  e.stopPropagation();
                  removeSameError(error.id);
                }}
                type="button"
                use:tooltip={{ message: '같은 단어 모두 무시하기', placement: 'top' }}
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
                _hover: { backgroundColor: 'interactive.hover', color: 'text.subtle' },
                _focusVisible: { backgroundColor: 'interactive.hover', color: 'text.subtle' },
              })}
              onclick={(e) => {
                e.stopPropagation();
                removeError(error.id);
              }}
              type="button"
              use:tooltip={{ message: '무시하기', placement: 'top' }}
            >
              <Icon icon={XIcon} size={14} />
            </button>
          </div>
          <div class={flex({ flexDirection: 'column', gap: '8px' })}>
            <div class={css({ fontSize: '14px', color: 'text.default' })}>{error.context}</div>

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
                    _hover: { backgroundColor: { base: 'red.100', _dark: 'dark.red.800' } },
                    _focusVisible: { backgroundColor: { base: 'red.100', _dark: 'dark.red.800' } },
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
                  _hover: { backgroundColor: 'surface.muted' },
                  _focusVisible: { backgroundColor: 'surface.muted' },
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
