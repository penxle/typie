import type { Readable } from 'svelte/store';
import type { $ArtifactSchema } from '../../../types';

export type FragmentStore<T extends $ArtifactSchema<'fragment'> | $ArtifactSchema<'fragment'>[]> = Readable<
  T extends (infer U extends $ArtifactSchema<'fragment'>)[] ? U['$output'][] : T extends $ArtifactSchema<'fragment'> ? T['$output'] : never
>;
