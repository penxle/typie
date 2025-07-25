import { Plugin, PluginKey } from '@tiptap/pm/state';
import { Decoration, DecorationSet } from '@tiptap/pm/view';
import { css } from '$styled-system/css';
import type { SpellingError } from './types';

export type SpellcheckPluginOptions = {
  onErrorClick?: (pos: number) => void;
  className?: string;
};

export const spellcheckKey = new PluginKey('spellcheck');

export function createSpellcheckPlugin(key: PluginKey, options?: SpellcheckPluginOptions) {
  const decorationClass =
    options?.className ||
    css({
      textDecoration: 'underline',
      textDecorationColor: 'text.danger',
      textDecorationStyle: 'wavy',
      textUnderlineOffset: '2px',
    });

  return new Plugin({
    key,
    state: {
      init: () => DecorationSet.empty,
      apply: (tr, state, _, newState) => {
        const meta = tr.getMeta(key) as SpellingError[] | undefined;
        if (meta) {
          const decorations: Decoration[] = [];
          for (const error of meta) {
            const decoration = Decoration.inline(error.from, error.to, {
              class: decorationClass,
              nodeName: 'span',
              'data-spellcheck-error': 'true',
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
      handleClick: options?.onErrorClick
        ? (_, pos, event) => {
            const target = event.target as HTMLElement;
            if (target.dataset.spellcheckError) {
              options.onErrorClick?.(pos);
              return true;
            }
            return false;
          }
        : undefined,
    },
  });
}
