import stringify from 'fast-json-stable-stringify';
import { nanoid } from 'nanoid';
import { rapidhash } from 'rapidhash-js';
import { make, makeSubject, pipe, subscribe } from 'wonka';
import { denormalize } from './denormalize';
import { normalize } from './normalize';
import { RootFieldKey } from './types';
import { deepMerge, entries, makeDependencyKey, makeQueryKey } from './utils';
import type { Source, Subject } from 'wonka';
import type { $ArtifactSchema, ArtifactSchema } from '../../types';
import type { Data, DependencyKey, EntityKey, FieldKey, QueryKey, Storage, StorageKey, Variables } from './types';

type Path = `${EntityKey}.${FieldKey}` | `Query.${FieldKey}`;
type Query = {
  schema: ArtifactSchema;
  paths: Set<Path>;
  variables: Variables;
};

type QueryResult<T extends $ArtifactSchema = $ArtifactSchema> = { data: T['$output']; partial: boolean };

export class Cache {
  #storage: Storage = { [RootFieldKey]: {} };
  #optimisticLayers = new Map<string, Storage>();
  #queries = new Map<QueryKey, Query>();
  #dependencies = new Map<DependencyKey, Set<QueryKey>>();
  #subjects = new Map<QueryKey, Subject<QueryResult>>();
  #lastResultHashes = new Map<QueryKey, string>();
  #refetchPromises = new Map<QueryKey, { resolve: () => void; reject: (err: Error) => void }[]>();
  id = nanoid();

  readQuery<T extends $ArtifactSchema>(schema: ArtifactSchema<T>, variables: Variables): T['$output'] | null {
    const result = this.#readQuery(schema, variables);
    if (result.partial) {
      return null;
    }

    return result.data;
  }

  writeQuery<T extends $ArtifactSchema>(schema: ArtifactSchema<T>, variables: Variables, data: T['$output']) {
    const queryKey = makeQueryKey(schema, variables);

    try {
      const normalized = normalize(schema, variables, data as Data);

      const fieldUpdates = new Map<EntityKey, Set<FieldKey>>();

      let isStorageChanged = false;

      for (const [key, value] of entries(normalized)) {
        if (key === RootFieldKey) {
          this.#storage[RootFieldKey] = deepMerge(this.#storage[RootFieldKey], value, { arrayStrategy: 'replace' });
          continue;
        }

        if (this.#storage[key]) {
          const updatedFields = new Set<FieldKey>(Object.keys(value).filter((field) => this.#storage[key][field] !== value[field]));
          if (updatedFields.size > 0) {
            fieldUpdates.set(key, updatedFields);
            isStorageChanged = true;
          }

          this.#storage[key] = deepMerge(this.#storage[key], value, { arrayStrategy: 'replace' });
        } else {
          this.#storage[key] = value;
          fieldUpdates.set(key, new Set(Object.keys(value)));
          isStorageChanged = true;
        }
      }

      if (isStorageChanged) {
        this.#refreshAffectedQueries(fieldUpdates, queryKey);
      }

      if (this.#queries.has(queryKey)) {
        const query = this.#queries.get(queryKey);
        query?.paths.clear();
      } else {
        this.#queries.set(queryKey, { paths: new Set(), schema, variables });
      }

      const result = denormalize(schema, variables, this.#storage, (storageKey, fieldKey) => {
        this.#trackDependency(queryKey, storageKey, fieldKey);
      });

      const resultHash = rapidhash(stringify(result.data)).toString();
      const lastHash = this.#lastResultHashes.get(queryKey);

      if (lastHash !== resultHash) {
        this.#lastResultHashes.set(queryKey, resultHash);

        const subject = this.#retriveSubject(queryKey);
        subject.next(result);
      }

      // Resolve refetch promises if data is complete
      if (!result.partial) {
        const promises = this.#refetchPromises.get(queryKey);
        if (promises && promises.length > 0) {
          for (const { resolve } of promises) {
            resolve();
          }
          this.#refetchPromises.delete(queryKey);
        }
      }
    } catch (err) {
      // Reject waiting refetch promises on error
      const promises = this.#refetchPromises.get(queryKey);
      if (promises && promises.length > 0) {
        for (const { reject } of promises) {
          reject(err as Error);
        }
        this.#refetchPromises.delete(queryKey);
      }
      throw err;
    }
  }

  writeFragment<T extends Record<string, unknown>>(entityKey: EntityKey, data: T) {
    this.#storage[entityKey] = deepMerge(this.#storage[entityKey] ?? {}, data, { arrayStrategy: 'merge' });

    const fieldUpdates = new Map<EntityKey, Set<FieldKey>>([[entityKey, new Set(Object.keys(data))]]);
    this.#refreshAffectedQueries(fieldUpdates);
  }

  observe<T extends $ArtifactSchema>(schema: ArtifactSchema<T>, variables: Variables): Source<QueryResult<T>> {
    const queryKey = makeQueryKey(schema, variables);
    const subject = this.#retriveSubject(queryKey);

    return make<QueryResult<T>>((observer) => {
      const result = this.#readQuery<T>(schema, variables);

      observer.next(result);

      const subscription = pipe(
        subject.source,
        subscribe((result) => {
          observer.next(result as QueryResult<T>);
        }),
      );

      return subscription.unsubscribe;
    });
  }

  invalidate(storageKey: StorageKey, fieldKey?: FieldKey): Set<QueryKey> {
    const affectedQueries = new Set<QueryKey>();

    if (fieldKey && this.#storage[storageKey] && typeof this.#storage[storageKey] === 'object') {
      // eslint-disable-next-line @typescript-eslint/no-dynamic-delete
      delete this.#storage[storageKey][fieldKey];

      if (storageKey === RootFieldKey) {
        const dependencyKey: DependencyKey = `Query:${fieldKey}`;
        const queries = this.#dependencies.get(dependencyKey) ?? new Set();

        for (const queryKey of queries) {
          affectedQueries.add(queryKey);
          this.#refreshQuery(queryKey);
        }
      } else {
        const fieldUpdates = new Map<EntityKey, Set<FieldKey>>([[storageKey as EntityKey, new Set([fieldKey])]]);
        const queries = this.#getAffectedQueries(fieldUpdates);
        for (const queryKey of queries) {
          affectedQueries.add(queryKey);
        }
        this.#refreshAffectedQueries(fieldUpdates);
      }
    } else if (!fieldKey) {
      // eslint-disable-next-line @typescript-eslint/no-dynamic-delete
      delete this.#storage[storageKey];

      if (storageKey === RootFieldKey) {
        this.#storage[RootFieldKey] = {};

        for (const queryKey of this.#queries.keys()) {
          affectedQueries.add(queryKey);
          this.#refreshQuery(queryKey);
        }
      } else {
        for (const [dependencyKey, queryKeys] of this.#dependencies.entries()) {
          if (dependencyKey.startsWith(`${storageKey}:`)) {
            for (const queryKey of queryKeys) {
              affectedQueries.add(queryKey);
            }
          }
        }

        for (const queryKey of affectedQueries) {
          this.#refreshQuery(queryKey);
        }
      }
    }

    return affectedQueries;
  }

  async waitForRefetches(queryKeys: Set<QueryKey>): Promise<void> {
    if (queryKeys.size === 0) {
      return;
    }

    const promises: Promise<void>[] = [];

    for (const queryKey of queryKeys) {
      const promise = new Promise<void>((resolve, reject) => {
        const existing = this.#refetchPromises.get(queryKey) ?? [];
        existing.push({ resolve, reject });
        this.#refetchPromises.set(queryKey, existing);
      });
      promises.push(promise);
    }

    await Promise.all(promises);
  }

  clear() {
    this.#storage = { [RootFieldKey]: {} };
    this.#queries.clear();
    this.#dependencies.clear();
    this.#lastResultHashes.clear();

    for (const [, subject] of this.#subjects) {
      subject.next({ data: null as unknown as Data, partial: true });
    }
  }

  addOptimisticLayer(key: string, schema: ArtifactSchema, variables: Variables, data: Data) {
    const normalized = normalize(schema, variables, data);
    this.#optimisticLayers.set(key, normalized);

    const fieldUpdates = this.#extractFieldUpdates(normalized);
    this.#refreshAffectedQueries(fieldUpdates);
  }

  removeOptimisticLayer(key: string) {
    const layer = this.#optimisticLayers.get(key);
    if (layer && this.#optimisticLayers.delete(key)) {
      const fieldUpdates = this.#extractFieldUpdates(layer);
      this.#refreshAffectedQueries(fieldUpdates);
    }
  }

  clearOptimisticLayers() {
    const allFieldUpdates = new Map<EntityKey, Set<FieldKey>>();

    for (const layer of this.#optimisticLayers.values()) {
      const fieldUpdates = this.#extractFieldUpdates(layer);
      for (const [entity, fields] of fieldUpdates) {
        const existing = allFieldUpdates.get(entity) ?? new Set();
        for (const field of fields) {
          existing.add(field);
        }
        allFieldUpdates.set(entity, existing);
      }
    }

    this.#optimisticLayers.clear();
    this.#refreshAffectedQueries(allFieldUpdates);
  }

  #readQuery<T extends $ArtifactSchema>(schema: ArtifactSchema<T>, variables: Variables): QueryResult<T> {
    const queryKey = makeQueryKey(schema, variables);

    if (!this.#queries.has(queryKey)) {
      this.#queries.set(queryKey, {
        paths: new Set(),
        schema,
        variables,
      });
    }

    const mergedStorage = this.#getMergedStorage();
    return denormalize(schema, variables, mergedStorage, (storageKey, fieldKey) => {
      this.#trackDependency(queryKey, storageKey, fieldKey);
    });
  }

  #trackDependency(queryKey: QueryKey, storageKey: StorageKey, fieldKey: FieldKey) {
    const dependency = this.#queries.get(queryKey);
    if (!dependency) {
      return;
    }

    if (storageKey === RootFieldKey) {
      dependency.paths.add(`Query.${fieldKey}`);

      const fieldDependencyKey: DependencyKey = `Query:${fieldKey}`;
      const fieldDependencies = this.#dependencies.get(fieldDependencyKey) ?? new Set();
      fieldDependencies.add(queryKey);
      this.#dependencies.set(fieldDependencyKey, fieldDependencies);
    } else {
      const entityKey = storageKey as EntityKey;
      dependency.paths.add(`${entityKey}.${fieldKey}`);

      const entityDependencyKey = makeDependencyKey(entityKey);
      const entityDependencies = this.#dependencies.get(entityDependencyKey) ?? new Set();
      entityDependencies.add(queryKey);
      this.#dependencies.set(entityDependencyKey, entityDependencies);

      const fieldDependencyKey = makeDependencyKey(entityKey, fieldKey);
      const fieldDependencies = this.#dependencies.get(fieldDependencyKey) ?? new Set();
      fieldDependencies.add(queryKey);
      this.#dependencies.set(fieldDependencyKey, fieldDependencies);
    }
  }

  #refreshQuery(queryKey: QueryKey) {
    const query = this.#queries.get(queryKey);
    if (!query) {
      return;
    }

    query.paths.clear();

    const mergedStorage = this.#getMergedStorage();
    const result = denormalize(query.schema, query.variables, mergedStorage, (storageKey, fieldKey) => {
      this.#trackDependency(queryKey, storageKey, fieldKey);
    });

    const resultHash = rapidhash(stringify(result.data)).toString();
    const lastHash = this.#lastResultHashes.get(queryKey);

    if (lastHash !== resultHash) {
      this.#lastResultHashes.set(queryKey, resultHash);
      const subject = this.#retriveSubject(queryKey);
      subject.next(result);
    }
  }

  #getAffectedQueries(fieldUpdates: Map<EntityKey, Set<FieldKey>>, excludeKey?: QueryKey): Set<QueryKey> {
    const queryKeys = new Set<QueryKey>();

    for (const [entity, fields] of fieldUpdates.entries()) {
      for (const field of fields) {
        if (field === '__typename') {
          continue;
        }

        const dependencies = this.#dependencies.get(makeDependencyKey(entity, field)) ?? new Set();
        for (const key of dependencies) {
          if (!excludeKey || key !== excludeKey) {
            const dependency = this.#queries.get(key);
            if (dependency && dependency.paths.has(`${entity}.${field}`)) {
              queryKeys.add(key);
            }
          }
        }
      }
    }

    return queryKeys;
  }

  #refreshAffectedQueries(fieldUpdates: Map<EntityKey, Set<FieldKey>>, excludeKey?: QueryKey) {
    const queryKeys = this.#getAffectedQueries(fieldUpdates, excludeKey);

    for (const queryKey of queryKeys) {
      this.#refreshQuery(queryKey);
    }
  }

  #getMergedStorage(): Storage {
    if (this.#optimisticLayers.size === 0) {
      return this.#storage;
    }

    const merged = { ...this.#storage };

    for (const layer of this.#optimisticLayers.values()) {
      for (const [key, value] of entries(layer)) {
        if (key === RootFieldKey) {
          merged[RootFieldKey] = deepMerge(merged[RootFieldKey] ?? {}, value, { arrayStrategy: 'replace' });
        } else if (merged[key]) {
          merged[key] = deepMerge(merged[key], value, { arrayStrategy: 'replace' });
        } else {
          merged[key] = value;
        }
      }
    }

    return merged;
  }

  #extractFieldUpdates(storage: Storage): Map<EntityKey, Set<FieldKey>> {
    const fieldUpdates = new Map<EntityKey, Set<FieldKey>>();

    for (const [key, value] of entries(storage)) {
      if (key === RootFieldKey) {
        continue;
      }

      const fields = new Set<FieldKey>(Object.keys(value));
      fieldUpdates.set(key as EntityKey, fields);
    }

    return fieldUpdates;
  }

  #retriveSubject(queryKey: QueryKey) {
    if (!this.#subjects.has(queryKey)) {
      this.#subjects.set(queryKey, makeSubject<QueryResult>());
    }

    return this.#subjects.get(queryKey) as Subject<QueryResult>;
  }
}

const cache = new Cache();
export const createCache = (): Cache => {
  if (typeof window === 'undefined') {
    return new Cache();
  }

  return cache;
};
