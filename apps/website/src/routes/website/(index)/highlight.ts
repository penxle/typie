import { Extension } from '@tiptap/core';
import { Plugin } from '@tiptap/pm/state';
import { Decoration, DecorationSet } from '@tiptap/pm/view';
import { nanoid } from 'nanoid';
import { generateRandomName } from '@/utils/name';
import { css } from '$styled-system/css';

const text = '글쓰기 앱을 만나보세요';

export const Highlight = Extension.create({
  name: 'highlight',

  addProseMirrorPlugins() {
    const name = generateRandomName(nanoid());
    let status = 'initial';

    return [
      new Plugin({
        props: {
          decorations: (state) => {
            const decorations: Decoration[] = [];
            const { doc } = state;

            doc.descendants((node, pos) => {
              if (!node.isText || status === 'miss') {
                return;
              }

              const index = node.text?.indexOf(text);
              if (index !== undefined && index !== -1) {
                decorations.push(
                  Decoration.inline(pos + index, pos + index + text.length, {
                    style: `--name: "${name}"`,
                    class: css({
                      position: 'relative',
                      display: 'inline-block',
                      paddingX: '8px',
                      pointerEvents: 'none',
                      _before: {
                        content: '""',
                        position: 'absolute',
                        left: '0',
                        top: '0',
                        width: '[calc(var(--highlight-progress) * 100%)]',
                        borderLeftRadius: '[calc(12px * var(--highlight-progress))]',
                        borderRightWidth: '4px',
                        borderRightColor: 'brand.500',
                        height: 'full',
                        backgroundColor: 'brand.500/15',
                        transitionProperty: 'width',
                        transitionDuration: '1s',
                        transitionTimingFunction: 'ease',
                        willChange: 'width',
                      },
                      _after: {
                        content: 'var(--name)',
                        position: 'absolute',
                        right: '[calc((1 - var(--highlight-progress)) * (100% - 4px))]',
                        top: '1px',
                        borderTopLeftRadius: '4px',
                        borderRightRadius: '4px',
                        paddingX: '8px',
                        paddingY: '4px',
                        width: 'max',
                        fontFamily: 'ui',
                        fontSize: '14px',
                        fontWeight: 'bold',
                        color: 'white',
                        backgroundColor: 'brand.500',
                        translate: 'auto',
                        translateX: '[calc(100% - 4px)]',
                        translateY: '-full',
                        transitionProperty: 'right',
                        transitionDuration: '1s',
                        transitionTimingFunction: 'ease',
                        willChange: 'right',
                      },
                    }),
                  }),
                );
              }
            });

            if (decorations.length > 0 && status === 'initial') {
              status = 'hit';
            }

            if (decorations.length === 0 && status === 'hit') {
              status = 'miss';
            }

            return DecorationSet.create(doc, decorations);
          },
        },
      }),
    ];
  },
});
