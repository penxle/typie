import { Node } from '@tiptap/core';
import { Plugin } from '@tiptap/pm/state';
import { Decoration, DecorationSet } from '@tiptap/pm/view';

declare module '@tiptap/core' {
  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Commands<ReturnType> {
    hardBreak: {
      setHardBreak: () => ReturnType;
    };
  }
}

export const HardBreak = Node.create({
  name: 'hard_break',
  group: 'inline',
  inline: true,
  selectable: false,
  linebreakReplacement: true,

  parseHTML() {
    return [{ tag: 'br' }];
  },

  renderHTML({ HTMLAttributes }) {
    return ['br', HTMLAttributes];
  },

  addCommands() {
    return {
      setHardBreak:
        () =>
        ({ commands, chain, state, editor }) => {
          return commands.first([
            () => commands.exitCode(),
            () =>
              commands.command(() => {
                const { selection, storedMarks } = state;

                if (selection.$from.parent.type.spec.isolating) {
                  return false;
                }

                const { splittableMarks } = editor.extensionManager;
                const marks = storedMarks || (selection.$to.parentOffset && selection.$from.marks());

                return chain()
                  .insertContent({ type: this.name })
                  .command(({ tr, dispatch }) => {
                    if (dispatch && marks) {
                      const filteredMarks = marks.filter((mark) => splittableMarks.includes(mark.type.name));

                      tr.ensureMarks(filteredMarks);
                    }

                    return true;
                  })
                  .run();
              }),
          ]);
        },
    };
  },

  addKeyboardShortcuts() {
    return {
      'Shift-Enter': () => this.editor.commands.setHardBreak(),
    };
  },

  addProseMirrorPlugins() {
    const editor = this.editor;

    return [
      new Plugin({
        props: {
          decorations(state) {
            const decorations: Decoration[] = [];
            const { doc } = state;

            const isPageMode = editor?.storage?.page.layout;

            if (!isPageMode) {
              return DecorationSet.create(doc, decorations);
            }

            // NOTE: 페이지 브레이크 대응
            doc.descendants((node, pos) => {
              if (node.type.name === 'hard_break') {
                const decoration = Decoration.widget(
                  pos + 1,
                  () => {
                    const span = document.createElement('span');
                    span.style.display = 'inline-block';
                    span.style.width = '1px';
                    span.style.height = '1em';
                    return span;
                  },
                  {
                    side: 1,
                    marks: [],
                  },
                );
                decorations.push(decoration);
              }
            });

            return DecorationSet.create(doc, decorations);
          },
        },
      }),
    ];
  },
});
