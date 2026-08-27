import { getContext, hasContext, setContext } from 'svelte';

const CONTEXT_KEY_PREFIX = 'typie.svelte-context.';

export const createStableContext = <T>(name: string) => {
  const key = Symbol.for(`${CONTEXT_KEY_PREFIX}${name}`);

  return [
    () => {
      if (!hasContext(key)) throw new Error(`${name} context was not set`);
      return getContext<T>(key);
    },
    (context: T) => setContext(key, context),
    () => getContext<T | undefined>(key),
  ] as const;
};
