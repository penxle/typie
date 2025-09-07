import { TextSelection } from '@tiptap/pm/state';
import { createNodeView } from '../../lib';
import Component from './Component.svelte';

declare module '@tiptap/core' {
  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Commands<ReturnType> {
    htmlBlock: {
      setHtmlBlock: () => ReturnType;
    };
  }
}

export const HtmlBlock = createNodeView(Component, {
  name: 'html_block',
  group: 'block',
  content: 'text*',
  marks: '',
  code: true,

  parseHTML() {
    return [{ tag: 'pre' }];
  },

  addCommands() {
    return {
      setHtmlBlock:
        () =>
        ({ can, chain }) => {
          if (!can().isNodeAllowed(this.name)) {
            return false;
          }

          return chain()
            .first(({ chain, commands }) => [
              () => commands.insertNodeWithSelection(this.name),
              () => {
                return chain()
                  .insertNode(this.type.create())
                  .command(({ tr, dispatch }) => {
                    // NOTE: 노드 삽입 후 내부로 커서 이동 (어째선지 code_block에서는 필요 없음)
                    const { $from } = tr.selection;
                    const pos = $from.pos + 1;

                    if (dispatch) {
                      tr.setSelection(TextSelection.create(tr.doc, pos));
                      dispatch(tr);
                    }

                    return true;
                  })
                  .run();
              },
            ])
            .run();
        },
    };
  },
});
