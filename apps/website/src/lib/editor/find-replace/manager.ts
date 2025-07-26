import { Decoration, DecorationSet } from '@tiptap/pm/view';
import { searchPluginKey } from '$lib/tiptap/extensions/search';
import { css } from '$styled-system/css';
import type { Editor } from '@tiptap/core';
import type { FindReplaceResult } from './types';

export type FindReplaceManager = {
  search: (text: string) => { results: FindReplaceResult[]; currentIndex: number };
  next: () => number;
  previous: () => number;
  replace: (replaceText: string) => { success: boolean; currentIndex: number };
  replaceAll: (replaceText: string) => number;
  clear: () => void;
  getCurrentMatch: () => number;
  getResults: () => FindReplaceResult[];
  getSearchText: () => string;
};

export function createFindReplaceManager(editor: Editor): FindReplaceManager {
  let results: FindReplaceResult[] = [];
  let currentIndex = 0;
  let searchText = '';

  const scrollTo = (pos: number) => {
    let { node: scrollEl } = editor.view.domAtPos(pos);
    if (scrollEl?.nodeType === Node.TEXT_NODE) {
      scrollEl = scrollEl.parentElement as HTMLElement;
    }
    if (scrollEl instanceof HTMLElement) {
      scrollEl.scrollIntoView({ block: 'center' });
    }
  };

  const clearDecorations = () => {
    const { state, dispatch } = editor.view;
    const tr = state.tr.setMeta(searchPluginKey, { decorations: DecorationSet.empty });
    dispatch(tr);
  };

  const updateDecorations = () => {
    if (!searchText) {
      clearDecorations();
      return;
    }

    const { state } = editor.view;
    const { doc } = state;
    const searchLower = searchText.toLowerCase();
    const decorations: Decoration[] = [];
    let matchIndex = 0;

    doc.descendants((node, pos) => {
      if (!node.isText || !node.text) return;

      const text = node.text.toLowerCase();
      let index = text.indexOf(searchLower);

      while (index !== -1) {
        const from = pos + index;
        const to = from + searchText.length;
        const isCurrentMatch = matchIndex === currentIndex;

        const className = css({
          color: '[#000]',
          backgroundColor: '[#ffd700]',
          '&[data-current-match="true"]': {
            color: '[#fff]',
            backgroundColor: '[#ff6b00]',
          },
        });

        decorations.push(
          Decoration.inline(from, to, {
            class: className,
            'data-current-match': isCurrentMatch ? 'true' : 'false',
          }),
        );

        matchIndex++;
        index = text.indexOf(searchLower, index + 1);
      }
    });

    const decorationSet = DecorationSet.create(doc, decorations);
    const { dispatch } = editor.view;
    const tr = state.tr.setMeta(searchPluginKey, { decorations: decorationSet });
    dispatch(tr);
  };

  const performSearch = (text: string): { results: FindReplaceResult[]; closestIndex: number } => {
    if (!text) return { results: [], closestIndex: 0 };

    const newResults: FindReplaceResult[] = [];
    const { view } = editor;
    const { doc, selection } = view.state;
    const searchLower = text.toLowerCase();
    let matchIndex = 0;
    let closestAfterIndex = -1;
    let closestAfterDistance = Infinity;
    let firstMatchIndex = -1;

    doc.descendants((node, pos) => {
      if (!node.isText || !node.text) return;

      const nodeText = node.text.toLowerCase();
      let index = nodeText.indexOf(searchLower);

      while (index !== -1) {
        const from = pos + index;
        const to = from + text.length;

        newResults.push({
          from,
          to,
          index: matchIndex,
        });

        if (firstMatchIndex === -1) {
          firstMatchIndex = matchIndex;
        }

        if (from >= selection.from) {
          const distance = from - selection.to;
          if (distance < closestAfterDistance) {
            closestAfterDistance = distance;
            closestAfterIndex = matchIndex;
          }
        }

        matchIndex++;
        index = nodeText.indexOf(searchLower, index + 1);
      }
    });

    let closestIndex = 0;
    if (newResults.length > 0) {
      if (closestAfterIndex !== -1) {
        closestIndex = closestAfterIndex;
      } else if (firstMatchIndex !== -1) {
        closestIndex = firstMatchIndex;
      }
      editor.commands.setTextSelection({ from: newResults[closestIndex].from, to: newResults[closestIndex].to });
    }

    return { results: newResults, closestIndex };
  };

  const selectMatch = (index: number) => {
    if (results.length > 0 && results[index]) {
      editor.commands.setTextSelection({
        from: results[index].from,
        to: results[index].to,
      });
      scrollTo(results[index].from);
    }
  };

  const search = (text: string) => {
    searchText = text;

    if (!text) {
      results = [];
      currentIndex = 0;
      clearDecorations();
      return { results: [], currentIndex: 0 };
    }

    const searchResult = performSearch(text);
    results = searchResult.results;
    currentIndex = searchResult.closestIndex;

    updateDecorations();
    selectMatch(currentIndex);

    return { results, currentIndex };
  };

  const next = () => {
    if (!searchText || results.length === 0) return currentIndex;

    const afterPos = editor.view.state.selection.to;
    let nextMatch = results.findIndex((result) => result.from >= afterPos);

    if (nextMatch === -1) {
      nextMatch = 0;
    }

    currentIndex = nextMatch;
    updateDecorations();
    selectMatch(currentIndex);

    return currentIndex;
  };

  const previous = () => {
    if (!searchText || results.length === 0) return currentIndex;

    const beforePos = editor.view.state.selection.from;
    let prevMatch = results.findLastIndex((result) => result.to <= beforePos);

    if (prevMatch === -1) {
      prevMatch = results.length - 1;
    }

    currentIndex = prevMatch;
    updateDecorations();
    selectMatch(currentIndex);

    return currentIndex;
  };

  const replace = (replaceText: string) => {
    if (!searchText || results.length === 0 || currentIndex < 0 || currentIndex >= results.length) {
      return { success: false, currentIndex };
    }

    const currentResult = results[currentIndex];
    const { selection } = editor.view.state;

    // Ensure we're at the correct match
    if (selection.from !== currentResult.from || selection.to !== currentResult.to) {
      selectMatch(currentIndex);
    }

    // Perform replacement
    editor.chain().insertContentAt({ from: currentResult.from, to: currentResult.to }, replaceText).run();

    // Re-search and find next match
    const newSearchResult = performSearch(searchText);
    results = newSearchResult.results;

    // Find the next match after the replaced position
    const replacedPos = currentResult.from + replaceText.length;
    let nextIndex = results.findIndex((result) => result.from >= replacedPos);

    if (nextIndex === -1) {
      nextIndex = 0;
    }

    currentIndex = nextIndex;
    updateDecorations();

    if (results.length > 0) {
      selectMatch(currentIndex);
    }

    return { success: true, currentIndex };
  };

  const replaceAll = (replaceText: string) => {
    if (!searchText) return 0;

    const searchResult = performSearch(searchText);
    if (searchResult.results.length === 0) return 0;

    const { view } = editor;
    const tr = view.state.tr;
    let offset = 0;

    searchResult.results.forEach((result) => {
      const from = result.from + offset;
      const to = result.to + offset;

      if (replaceText === '') {
        tr.delete(from, to);
      } else {
        tr.replaceWith(from, to, view.state.schema.text(replaceText));
      }

      offset += replaceText.length - searchText.length;
    });

    view.dispatch(tr);

    return searchResult.results.length;
  };

  const clear = () => {
    searchText = '';
    results = [];
    currentIndex = 0;
    clearDecorations();
  };

  return {
    search,
    next,
    previous,
    replace,
    replaceAll,
    clear,
    getCurrentMatch: () => currentIndex,
    getResults: () => results,
    getSearchText: () => searchText,
  };
}
