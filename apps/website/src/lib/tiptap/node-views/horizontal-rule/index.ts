import { createNodeView } from '$lib/tiptap/lib';
import { defaultValues, values } from '$lib/tiptap/values';
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
        isRequired: true,
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
        ({ commands }) => {
          return commands.insertContent({ type: this.name, attrs: { type } });
        },
    };
  },
});
