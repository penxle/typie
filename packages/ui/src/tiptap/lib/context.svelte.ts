import { getContext, setContext } from 'svelte';

const key: unique symbol = Symbol('EditorContext');

type EditorContext = {
  pdf: boolean;
};

export const getEditorContext = () => {
  return getContext<EditorContext>(key);
};

export const setupEditorContext = (value?: Partial<EditorContext>) => {
  const ctx = getContext<EditorContext | undefined>(key);
  if (ctx) {
    if (value) {
      Object.assign(ctx, value);
    }

    return ctx;
  } else {
    const context = $state<EditorContext>({
      pdf: false,
      ...value,
    });

    setContext(key, context);

    return context;
  }
};
