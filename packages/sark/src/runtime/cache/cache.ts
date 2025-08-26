import { nanoid } from 'nanoid';
import { make, makeSubject, pipe, subscribe } from 'wonka';
import { denormalize } from './denormalize';
import { normalize } from './normalize';
import { RootFieldKey } from './types';
import { deepMerge, entries, makeDependencyKey, makeQueryKey } from './utils';
import type { Source, Subject } from 'wonka';
import type { $ArtifactSchema, ArtifactSchema } from '../../types';
import type { Data, DependencyKey, EntityKey, FieldKey, QueryKey, Storage, Variables } from './types';

type Path = `${EntityKey}.${FieldKey}`;
type Query = {
  schema: ArtifactSchema;
  paths: Set<Path>;
  variables: Variables;
};

type QueryResult<T extends $ArtifactSchema = $ArtifactSchema> = { data: T['$output']; partial: boolean };

export class Cache {
  #storage: Storage = { [RootFieldKey]: {} };
  #queries = new Map<QueryKey, Query>();
  #dependencies = new Map<DependencyKey, Set<QueryKey>>();
  #subjects = new Map<QueryKey, Subject<QueryResult>>();
  #lastResults = new Map<QueryKey, string>();
  id = nanoid();

  readQuery<T extends $ArtifactSchema>(schema: ArtifactSchema<T>, variables: Variables): T['$output'] | null {
    const result = this.#readQuery(schema, variables);
    if (result.partial) {
      return null;
    }

    return result.data;
  }

  writeQuery<T extends $ArtifactSchema>(schema: ArtifactSchema<T>, variables: Variables, data: T['$output']) {
    const normalized = normalize(schema, variables, data as Data);

    const queryKey = makeQueryKey(schema, variables);
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

    const result = denormalize(schema, variables, this.#storage, (entityKey, fieldKey) => {
      this.#trackDependency(queryKey, entityKey, fieldKey);
    });

    const resultString = JSON.stringify(result.data);
    const lastResult = this.#lastResults.get(queryKey);

    if (lastResult !== resultString) {
      this.#lastResults.set(queryKey, resultString);

      const subject = this.#retriveSubject(queryKey);
      subject.next(result);
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

  invalidate(entityKey: EntityKey, fieldKey?: FieldKey) {
    if (fieldKey && this.#storage[entityKey] && typeof this.#storage[entityKey] === 'object') {
      // eslint-disable-next-line @typescript-eslint/no-dynamic-delete
      delete this.#storage[entityKey][fieldKey];

      const fieldUpdates = new Map<EntityKey, Set<FieldKey>>([[entityKey, new Set([fieldKey])]]);
      this.#refreshAffectedQueries(fieldUpdates);
    } else if (!fieldKey) {
      // eslint-disable-next-line @typescript-eslint/no-dynamic-delete
      delete this.#storage[entityKey];

      const affectedQueries = new Set<QueryKey>();

      for (const [dependencyKey, queryKeys] of this.#dependencies.entries()) {
        if (dependencyKey.startsWith(`${entityKey}:`)) {
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

  clear() {
    this.#storage = { [RootFieldKey]: {} };
    this.#queries.clear();
    this.#dependencies.clear();
    this.#lastResults.clear();

    for (const [, subject] of this.#subjects) {
      subject.next({ data: null as unknown as Data, partial: true });
    }
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

    return denormalize(schema, variables, this.#storage, (entityKey, fieldKey) => {
      this.#trackDependency(queryKey, entityKey, fieldKey);
    });
  }

  #trackDependency(queryKey: QueryKey, entityKey: EntityKey, fieldKey: FieldKey) {
    const dependency = this.#queries.get(queryKey);
    if (!dependency) {
      return;
    }

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

  #refreshQuery(queryKey: QueryKey) {
    const query = this.#queries.get(queryKey);
    if (!query) {
      return;
    }

    query.paths.clear();

    const result = denormalize(query.schema, query.variables, this.#storage, (entityKey, fieldKey) => {
      this.#trackDependency(queryKey, entityKey, fieldKey);
    });

    const resultString = JSON.stringify(result.data);
    const lastResult = this.#lastResults.get(queryKey);

    if (lastResult !== resultString) {
      this.#lastResults.set(queryKey, resultString);
      const subject = this.#retriveSubject(queryKey);
      subject.next(result);
    }
  }

  #refreshAffectedQueries(fieldUpdates: Map<EntityKey, Set<FieldKey>>, excludeKey?: QueryKey) {
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

    for (const queryKey of queryKeys) {
      this.#refreshQuery(queryKey);
    }
  }

  #retriveSubject(queryKey: QueryKey) {
    if (!this.#subjects.has(queryKey)) {
      this.#subjects.set(queryKey, makeSubject<QueryResult>());
    }

    return this.#subjects.get(queryKey) as Subject<QueryResult>;
  }
}

export const createCache = (): Cache => new Cache();
