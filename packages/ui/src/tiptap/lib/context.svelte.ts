import { getContext, setContext } from 'svelte';

const key: unique symbol = Symbol('EditorContext');

type EditorContext = {
  pdf: boolean;
  timeline: boolean;
};

export const getEditorContext = () => {
  return getContext<EditorContext | undefined>(key);
};

export const setupEditorContext = (value?: Partial<EditorContext>) => {
  const context = $state<EditorContext>({
    pdf: false,
    timeline: false,
    ...value,
  });

  setContext(key, context);

  return context;
};
