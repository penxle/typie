<script lang="ts">
  import { Decoration, DecorationSet } from '@tiptap/pm/view';
  import { onDestroy, onMount, tick } from 'svelte';
  import ArrowDownIcon from '~icons/lucide/arrow-down';
  import ArrowUpIcon from '~icons/lucide/arrow-up';
  import ReplaceIcon from '~icons/lucide/replace';
  import ReplaceAllIcon from '~icons/lucide/replace-all';
  import { tooltip } from '$lib/actions';
  import { Icon } from '$lib/components';
  import { searchPluginKey } from '$lib/tiptap/extensions/search';
  import { css } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';
  import type { Editor } from '@tiptap/core';
  import type { Ref } from '$lib/utils';

  type Props = {
    editor: Ref<Editor>;
    close: () => void;
  };

  let { editor, close }: Props = $props();

  let findInput: HTMLInputElement;
  let findText = $state('');
  let replaceText = $state('');
  let currentMatch = $state(0);
  let totalMatches = $state(0);

  const clearDecorations = () => {
    const { state, dispatch } = editor.current.view;
    const tr = state.tr.setMeta(searchPluginKey, { decorations: DecorationSet.empty });
    dispatch(tr);
  };

  const updateDecorations = () => {
    if (!findText) {
      clearDecorations();
      return;
    }

    const { state } = editor.current;
    const { doc } = state;
    const searchText = findText.toLowerCase();
    const decorations: Decoration[] = [];
    let matchIndex = 0;

    doc.descendants((node, pos) => {
      if (!node.isText || !node.text) return;

      const text = node.text.toLowerCase();
      let index = text.indexOf(searchText);

      while (index !== -1) {
        matchIndex++;
        const from = pos + index;
        const to = from + findText.length;

        const isCurrentMatch = matchIndex === currentMatch;
        const className = css({
          backgroundColor: isCurrentMatch ? '[#ff6b00]' : '[#ffd700]',
          color: isCurrentMatch ? '[#fff]' : '[#000]',
        });
        decorations.push(Decoration.inline(from, to, { class: className }));

        index = text.indexOf(searchText, index + 1);
      }
    });

    const decorationSet = DecorationSet.create(doc, decorations);
    const { dispatch } = editor.current.view;
    const tr = state.tr.setMeta(searchPluginKey, { decorations: decorationSet });
    dispatch(tr);
  };

  const updateMatches = () => {
    if (!findText) {
      totalMatches = 0;
      currentMatch = 0;
      clearDecorations();
      return;
    }

    const { state } = editor.current;
    const { doc, selection } = state;
    const searchText = findText.toLowerCase();
    let matches = 0;
    let matchIndex = 0;
    let closestMatchIndex = 0;
    let closestDistance = Infinity;

    doc.descendants((node, pos) => {
      if (node.isText && node.text) {
        const text = node.text.toLowerCase();
        let index = text.indexOf(searchText);
        while (index !== -1) {
          matches++;
          matchIndex++;

          // 현재 커서 위치와의 거리 계산
          const matchPos = pos + index;
          const distance = Math.abs(matchPos - selection.from);

          // 가장 가까운 매치 찾기
          if (distance < closestDistance) {
            closestDistance = distance;
            closestMatchIndex = matchIndex;
          }

          index = text.indexOf(searchText, index + 1);
        }
      }
    });

    totalMatches = matches;

    // 현재 커서 위치에서 가장 가까운 매치를 currentMatch로 설정
    if (matches > 0) {
      currentMatch = closestMatchIndex || 1;
    } else {
      currentMatch = 0;
    }

    updateDecorations();
  };

  const scrollToSelection = () => {
    const { state } = editor.current;
    const { selection } = state;
    let { node: scrollEl } = editor.current.view.domAtPos(selection.from);

    if (scrollEl?.nodeType === Node.TEXT_NODE) {
      scrollEl = scrollEl.parentElement as HTMLElement;
    }

    if (scrollEl instanceof HTMLElement) {
      scrollEl.scrollIntoView({ block: 'nearest' });
    }
  };

  type Match = { from: number; to: number; index: number };

  const findNext = () => {
    if (!findText || totalMatches === 0) return;

    const { state } = editor.current;
    const { doc, selection } = state;
    const searchText = findText.toLowerCase();
    let matchIndex = 0;
    let firstMatch: Match | null = null;
    let nextMatch: Match | null = null;

    doc.descendants((node, pos) => {
      if (!node.isText || !node.text) return;

      const text = node.text.toLowerCase();
      let index = text.indexOf(searchText);

      while (index !== -1) {
        matchIndex++;
        const from = pos + index;
        const to = from + findText.length;

        if (!firstMatch) {
          firstMatch = { from, to, index: matchIndex };
        }

        if (from >= selection.to && !nextMatch) {
          nextMatch = { from, to, index: matchIndex };
        }

        index = text.indexOf(searchText, index + 1);
      }
    });

    // 다음 매치가 있으면 그걸로, 없으면 처음으로
    const match = nextMatch || firstMatch;
    if (match) {
      editor.current.commands.setTextSelection({ from: (match as Match).from, to: (match as Match).to });
      currentMatch = (match as Match).index;
      updateDecorations();
      scrollToSelection();
    }
  };

  const findPrevious = () => {
    if (!findText || totalMatches === 0) return;

    const { state } = editor.current;
    const { doc, selection } = state;
    const searchText = findText.toLowerCase();
    let matchIndex = 0;
    let lastMatch: Match | null = null;

    doc.descendants((node, pos) => {
      if (!node.isText || !node.text) return;

      const text = node.text.toLowerCase();
      let index = text.indexOf(searchText);

      while (index !== -1) {
        matchIndex++;
        const from = pos + index;

        if (from < selection.from) {
          lastMatch = { from, to: from + findText.length, index: matchIndex };
        }

        index = text.indexOf(searchText, index + 1);
      }
    });

    if (lastMatch) {
      editor.current.commands.setTextSelection({ from: (lastMatch as Match).from, to: (lastMatch as Match).to });
      currentMatch = (lastMatch as Match).index;
      updateDecorations();
      scrollToSelection();
    } else if (totalMatches > 0) {
      // 처음에 있으면 맨 끝으로
      matchIndex = 0;
      doc.descendants((node, pos) => {
        if (!node.isText || !node.text) return;

        const text = node.text.toLowerCase();
        let index = text.indexOf(searchText);

        while (index !== -1) {
          matchIndex++;
          lastMatch = { from: pos + index, to: pos + index + findText.length, index: matchIndex };
          index = text.indexOf(searchText, index + 1);
        }
      });

      if (lastMatch) {
        editor.current.commands.setTextSelection({ from: (lastMatch as Match).from, to: (lastMatch as Match).to });
        currentMatch = (lastMatch as Match).index;
        updateDecorations();
        scrollToSelection();
      }
    }
  };

  const replace = () => {
    if (!findText || totalMatches === 0) return;

    // 현재 선택된 텍스트가 검색어와 일치하는지 확인
    const { state } = editor.current;
    const { selection } = state;
    const selectedText = state.doc.textBetween(selection.from, selection.to);

    if (selectedText.toLowerCase() === findText.toLowerCase()) {
      // 선택된 텍스트 바꾸기
      editor.current.chain().insertContentAt(selection, replaceText).run();

      updateMatches();
      findNext();
    } else if (currentMatch > 0) {
      // 선택이 없거나 일치하지 않으면 현재 매치로 이동 후 바꾸기
      findNext();
      replace();
    }
  };

  const replaceAll = () => {
    if (!findText) return;

    const { state } = editor.current;
    const { doc, tr } = state;
    const searchText = findText.toLowerCase();
    let offset = 0;
    let lastReplacedPos: number | null = null;

    doc.descendants((node, pos) => {
      if (!node.isText || !node.text) return;

      const text = node.text;
      const lowerText = text.toLowerCase();
      let index = lowerText.indexOf(searchText);

      while (index !== -1) {
        const from = pos + index + offset;
        const to = from + findText.length;

        // 빈 문자열로 바꾸는 경우 처리
        if (replaceText === '') {
          tr.delete(from, to);
        } else {
          tr.replaceWith(from, to, state.schema.text(replaceText));
        }

        lastReplacedPos = from;

        offset += replaceText.length - findText.length;
        index = lowerText.indexOf(searchText, index + 1);
      }
    });

    editor.current.view.dispatch(tr);

    // 마지막 바뀐 위치로 스크롤
    if (lastReplacedPos !== null) {
      editor.current.commands.setTextSelection({
        from: lastReplacedPos,
        to: lastReplacedPos + replaceText.length,
      });
      scrollToSelection();
    }

    updateMatches();
  };

  const handleKeydown = (e: KeyboardEvent) => {
    if (e.key === 'Escape') {
      close();
      editor.current.commands.focus(undefined, { scrollIntoView: false });
    }
  };

  const handleKeydownInFindInput = (e: KeyboardEvent) => {
    if (e.key === 'Enter') {
      if (e.shiftKey) {
        findPrevious();
      } else {
        findNext();
      }
    }
  };

  const handleKeydownInReplaceInput = (e: KeyboardEvent) => {
    if (e.key === 'Enter') {
      if (e.metaKey) {
        replaceAll();
      } else {
        replace();
      }
    }
  };

  onMount(() => {
    // 다음 틱에서 포커스 설정
    tick().then(() => {
      findInput?.focus();
    });
  });

  onDestroy(() => {
    clearDecorations();
  });

  $effect(() => {
    updateMatches();
  });
</script>

<div
  class={flex({
    gap: '4px',
    padding: '8px',
    width: '320px',
  })}
  onkeydown={handleKeydown}
  role="dialog"
  tabindex="-1"
>
  <div class={flex({ flexDirection: 'column', gap: '4px' })}>
    <input
      bind:this={findInput}
      id="find-input"
      class={css({
        paddingX: '8px',
        paddingY: '4px',
        height: '30px',
        borderWidth: '1px',
        borderColor: 'border.default',
        borderRadius: '6px',
        fontSize: '14px',
      })}
      aria-label="찾을 텍스트"
      onkeydown={handleKeydownInFindInput}
      placeholder="찾기"
      type="text"
      bind:value={findText}
    />
    <input
      id="replace-input"
      class={css({
        paddingX: '8px',
        paddingY: '4px',
        height: '30px',
        borderWidth: '1px',
        borderColor: 'border.default',
        borderRadius: '6px',
        fontSize: '14px',
      })}
      aria-label="바꿀 텍스트"
      onkeydown={handleKeydownInReplaceInput}
      placeholder="바꾸기"
      type="text"
      bind:value={replaceText}
    />
  </div>

  <div class={flex({ flex: '1', flexDirection: 'column', gap: '4px' })}>
    <div class={flex({ alignItems: 'center', gap: '4px', height: '30px' })}>
      <div class={css({ flex: '1', fontSize: '12px', color: 'text.subtle' })}>
        {#if totalMatches > 0}
          {currentMatch} / {totalMatches}
        {:else}
          결과 없음
        {/if}
      </div>

      <div class={flex({ gap: '4px' })}>
        <button
          class={css({
            size: '24px',
            padding: '4px',
            borderRadius: '4px',
            color: 'text.faint',
            _hover: {
              backgroundColor: 'surface.muted',
            },
            _disabled: {
              opacity: '[0.5]',
              _hover: {
                backgroundColor: 'transparent',
              },
            },
          })}
          disabled={!findText}
          onclick={findPrevious}
          type="button"
          use:tooltip={{
            message: '이전 결과 찾기',
            keys: ['Shift', 'Enter'],
          }}
        >
          <Icon icon={ArrowUpIcon} size={16} />
        </button>
        <button
          class={center({
            size: '24px',
            padding: '4px',
            borderRadius: '4px',
            color: 'text.faint',
            _hover: {
              backgroundColor: 'surface.muted',
            },
            _disabled: {
              opacity: '[0.5]',
              _hover: {
                backgroundColor: 'transparent',
              },
            },
          })}
          disabled={!findText}
          onclick={findNext}
          type="button"
          use:tooltip={{
            message: '다음 결과 찾기',
            keys: ['Enter'],
          }}
        >
          <Icon icon={ArrowDownIcon} size={16} />
        </button>
      </div>
    </div>
    <div class={flex({ alignItems: 'center', gap: '4px', height: '30px' })}>
      <div class={flex({ alignItems: 'center', gap: '4px' })}>
        <button
          class={center({
            size: '24px',
            padding: '4px',
            borderRadius: '4px',
            color: 'text.faint',
            _hover: {
              backgroundColor: 'surface.muted',
            },
            _disabled: {
              opacity: '[0.5]',
              _hover: {
                backgroundColor: 'transparent',
              },
            },
          })}
          disabled={!findText}
          onclick={() => replace()}
          type="button"
          use:tooltip={{
            message: '바꾸기',
            keys: ['Enter'],
          }}
        >
          <Icon icon={ReplaceIcon} size={16} />
        </button>
        <button
          class={center({
            size: '24px',
            padding: '4px',
            borderRadius: '4px',
            color: 'text.faint',
            _hover: {
              backgroundColor: 'surface.muted',
            },
            _disabled: {
              opacity: '[0.5]',
              _hover: {
                backgroundColor: 'transparent',
              },
            },
          })}
          disabled={!findText}
          onclick={replaceAll}
          type="button"
          use:tooltip={{
            message: '모두 바꾸기',
            keys: ['Mod', 'Enter'],
          }}
        >
          <Icon icon={ReplaceAllIcon} size={16} />
        </button>
      </div>
    </div>
  </div>
</div>
