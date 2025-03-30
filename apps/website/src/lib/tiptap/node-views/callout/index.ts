import { defaultValues, values } from '$lib/tiptap/values';
import { createNodeView } from '../../lib';
import Component from './Component.svelte';

const callouts = values.callout.map(({ type }) => type);
type Callout = (typeof callouts)[number];

declare module '@tiptap/core' {
  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Commands<ReturnType> {
    callout: {
      setCallout: () => ReturnType;
      toggleCallout: () => ReturnType;
      unsetCallout: () => ReturnType;
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
      setCallout:
        () =>
        ({ commands }) => {
          return commands.wrapIn(this.name);
        },
      toggleCallout:
        () =>
        ({ commands }) => {
          return commands.toggleWrap(this.name);
        },
      unsetCallout:
        () =>
        ({ commands }) => {
          return commands.lift(this.name);
        },
    };
  },
});
