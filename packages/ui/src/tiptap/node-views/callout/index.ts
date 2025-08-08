import { createNodeView } from '../../lib';
import { defaultValues, values } from '../../values';
import Component from './Component.svelte';

const callouts = values.callout.map(({ type }) => type);
type Callout = (typeof callouts)[number];

declare module '@tiptap/core' {
  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Commands<ReturnType> {
    callout: {
      toggleCallout: () => ReturnType;
    };
  }
}

export const Callout = createNodeView(Component, {
  name: 'callout',
  group: 'block',
  content: 'paragraph+',
  defining: true,

  addAttributes() {
    return {
      type: {
        default: 'info',
        parseHTML: (element) => {
          const callout = element.dataset.type;

          if (callout && (callouts as string[]).includes(callout)) {
            return callout;
          }

          return defaultValues.callout;
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
      toggleCallout:
        () =>
        ({ editor, commands }) => {
          if (editor.isActive(this.name)) {
            return commands.lift(this.name);
          } else {
            return commands.wrapIn(this.name);
          }
        },
    };
  },
});
