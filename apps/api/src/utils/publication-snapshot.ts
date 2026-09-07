import { TypieError } from '@typie/lib/errors';
import { wasm } from './wasm-ffi.ts';
import type { PlainDoc } from '@typie/editor-ffi/server';

export type PublicationSnapshot = {
  graph: Uint8Array;
  plain: PlainDoc;
  text: string;
  heads: Uint8Array;
  characterCount: number;
};

export const buildPublicationSnapshot = async (graph: Uint8Array): Promise<PublicationSnapshot> =>
  await wasm.use((host) => {
    const materialized = host.materialize(graph);
    if (materialized.projection_degraded) {
      throw new TypieError({ code: 'publication_projection_degraded', status: 409 });
    }
    return {
      graph: host.to_graph(materialized.plain),
      plain: materialized.plain,
      text: materialized.text,
      heads: host.heads(graph),
      characterCount: host.count_characters(materialized.text),
    };
  });
