import type { LoadEvent } from '@sveltejs/kit';
import type { $ArtifactSchema } from '../../types';

type Awaitable<T> = T | Promise<T>;

export type VariablesFn<Event extends LoadEvent, $Schema extends $ArtifactSchema> = (event: Event) => Awaitable<$Schema['$input']>;

export type AfterLoadFn<Event extends LoadEvent, $Schema extends $ArtifactSchema> = (params: {
  query: $Schema['$output'];
  event: Event;
}) => Awaitable<void>;

export type OnErrorFn<Event extends LoadEvent> = (params: { error: unknown; event: Event }) => Awaitable<void>;
