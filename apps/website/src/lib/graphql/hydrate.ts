import { createQuery, getClient } from '@mearie/svelte';
import type { Artifact } from '@mearie/svelte';
import type { HydratableQuery } from './server';

export function hydrateQuery<T extends Artifact<'query'>>(getHydratable: () => HydratableQuery<T>) {
  const hydratable = getHydratable();
  const { artifact, variables, cacheSnapshot } = hydratable.__hydration;

  const client = getClient();
  if (cacheSnapshot) {
    client.maybeExtension('cache')?.hydrate(cacheSnapshot);
  }

  return createQuery(
    artifact,
    () => variables,
    () => ({
      initialData: hydratable.data,
    }),
  );
}
