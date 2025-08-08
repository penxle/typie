import { createNodeView } from '../../lib';
import { defaultValues, values } from '../../values';
import Component from './Component.svelte';

const horizontalRules = values.horizontalRule.map(({ type }) => type);
type HorizontalRule = (typeof horizontalRules)[number];

declare module '@tiptap/core' {
  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Commands<ReturnType> {
    horizontalRule: {
      setHorizontalRule: (type?: HorizontalRule) => ReturnType;
    };
  }
}

export const HorizontalRule = createNodeView(Component, {
  name: 'horizontal_rule',
  group: 'block',

  addAttributes() {
    return {
      type: {
        default: defaultValues.horizontalRule,
        parseHTML: (element) => {
          const horizontalRule = element.dataset.type;

          if (horizontalRule && (horizontalRules as string[]).includes(horizontalRule)) {
            return horizontalRule;
          }

          return defaultValues.horizontalRule;
        },
        renderHTML: ({ type }) => {
          return {
            'data-type': type,
          };
        },
      },
    };
  },

  addCommands() {
    return {
      setHorizontalRule:
        (type) =>
        ({ can, editor, commands }) => {
          if (editor.isActive(this.name)) {
            return commands.updateAttributes(this.name, { type });
          } else {
            if (!can().isNodeAllowed(this.name)) {
              return false;
            }

            return commands.insertNode(this.type.create({ type }));
          }
        },
    };
  },
});
