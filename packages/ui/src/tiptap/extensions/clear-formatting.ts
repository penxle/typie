import { Extension } from '@tiptap/core';
import { NodeSelection } from '@tiptap/pm/state';
import { MultiNodeSelection } from '../extensions/selection';
import { defaultValues } from '../values';

declare module '@tiptap/core' {
  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Commands<ReturnType> {
    clearFormatting: {
      clearFormatting: () => ReturnType;
    };
  }
}

export const ClearFormatting = Extension.create({
  name: 'clear_formatting',

  addCommands() {
    return {
      clearFormatting:
        () =>
        ({ chain, state }) => {
          const { selection } = state;
          const { from, to } = selection;
          const $from = state.doc.resolve(from);
          const $to = state.doc.resolve(to);

          const isParagraphSelected =
            selection instanceof NodeSelection ||
            selection instanceof MultiNodeSelection ||
            ($from.parent.type.name === 'paragraph' && $to.parent.type.name === 'paragraph' && from === $from.start() && to === $to.end());

          const commands = chain()
            .unsetAllMarks()
            .setTextColor(defaultValues.textColor)
            .setTextBackgroundColor(defaultValues.textBackgroundColor)
            .setFontFamily(defaultValues.fontFamily)
            .setFontWeight(defaultValues.fontWeight)
            .setFontSize(defaultValues.fontSize);

          // NOTE: 문단 전체가 선택되었을 때만 문단 속성 리셋
          if (isParagraphSelected) {
            commands
              .setParagraphTextAlign(defaultValues.textAlign)
              .setParagraphLineHeight(defaultValues.lineHeight)
              .setParagraphLetterSpacing(defaultValues.letterSpacing);
          }

          return commands.run();
        },
    };
  },

  addKeyboardShortcuts() {
    return {
      'Mod-\\': () => this.editor.commands.clearFormatting(),
    };
  },
});
