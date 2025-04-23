import type { Readable } from 'svelte/store';
import type { $ArtifactSchema } from '../../../types';

type FragmentKey = ` $$_${string}`;
type FragmentRef<T = unknown> = Record<FragmentKey, T>;

export type Optional<T extends FragmentRef> = (T & { readonly __optional?: unique symbol }) | null | undefined;
export type List<T extends FragmentRef> = T[] & { readonly __list?: unique symbol };

type OutputOf<S> = S extends { $output: infer O } ? O : never;

export type FragmentStore<
  T extends $ArtifactSchema<'fragment'> | List<$ArtifactSchema<'fragment'>> | Optional<$ArtifactSchema<'fragment'>>,
> = Readable<
  T extends $ArtifactSchema<'fragment'>
    ? OutputOf<T>
    : T extends List<infer U>
      ? OutputOf<U>[]
      : T extends Optional<infer U>
        ? OutputOf<U> | null | undefined
        : never
>;

export type FragmentType<T extends FragmentRef> = T extends FragmentRef<infer U> ? OutputOf<U> : never;
