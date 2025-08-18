import { IndexeddbPersistence } from 'y-indexeddb';
import * as Y from 'yjs';

export async function getPostYjsAttr<T = unknown>(postId: string, attrName: string): Promise<T | undefined> {
  const doc = new Y.Doc();
  const persistence = new IndexeddbPersistence(`typie:editor:${postId}`, doc);

  try {
    await new Promise<void>((resolve) => {
      const timeout = setTimeout(() => resolve(), 3000);
      persistence.once('synced', () => {
        clearTimeout(timeout);
        resolve();
      });
    });

    const attrsMap = doc.getMap('attrs');
    return attrsMap.get(attrName) as T | undefined;
  } finally {
    persistence.destroy();
    doc.destroy();
  }
}

export async function getPostYjsAttrs<T extends Record<string, unknown>>(postId: string, attrNames: string[]): Promise<Partial<T>> {
  const doc = new Y.Doc();
  const persistence = new IndexeddbPersistence(`typie:editor:${postId}`, doc);

  try {
    await new Promise<void>((resolve) => {
      const timeout = setTimeout(() => resolve(), 3000);
      persistence.once('synced', () => {
        clearTimeout(timeout);
        resolve();
      });
    });

    const attrsMap = doc.getMap('attrs');
    const result: Partial<T> = {};

    for (const attrName of attrNames) {
      const value = attrsMap.get(attrName);
      if (value !== undefined) {
        result[attrName as keyof T] = value as T[keyof T];
      }
    }

    return result;
  } finally {
    persistence.destroy();
    doc.destroy();
  }
}
