import path from 'node:path';
import { getAllArtifacts, getArtifacts, getSchema } from '../artifact';
import { writeArtifactAssets, writeMiscAssets, writePublicAssets, writeTypeAssets } from '../codegen/writer';
import { transformGraphQL, transformLoad } from './transform';
import type * as graphql from 'graphql';
import type { Plugin } from 'vite';
import type { Artifact } from '../../types';

type SarkOptions = {
  schemaPath?: string;
  outDir?: string;
};

export const sark = (options?: SarkOptions): Plugin => {
  const { schemaPath = 'schema.graphql', outDir = '.sark' } = options ?? {};

  let projectDir: string;
  let resolvedOutDir: string;
  let resolvedSchemaPath: string;

  let schema: { schema: graphql.GraphQLSchema; hash: bigint };
  let artifacts: Artifact[] = [];

  const writeArtifacts = async () => {
    await writeArtifactAssets(resolvedOutDir, schema.schema, artifacts);
    await writePublicAssets(resolvedOutDir, artifacts);
    await writeTypeAssets(resolvedOutDir, projectDir, artifacts);
    await writeMiscAssets(resolvedOutDir);
  };

  return {
    name: '@typie/sark',

    configResolved(config) {
      projectDir = config.root;

      resolvedOutDir = path.join(projectDir, outDir);
      resolvedSchemaPath = path.join(projectDir, schemaPath);

      config.server.fs.allow.push(resolvedOutDir);
    },

    async buildStart() {
      schema = await getSchema(resolvedSchemaPath);
      artifacts = await getAllArtifacts(schema.schema, projectDir);
      await writeArtifacts();
    },

    async watchChange(id, { event }) {
      if (id === resolvedSchemaPath) {
        const newSchema = await getSchema(resolvedSchemaPath);
        if (schema.hash !== newSchema.hash) {
          console.log('😼 Schema change detected');
          schema = newSchema;
          artifacts = await getAllArtifacts(schema.schema, projectDir);
          await writeArtifacts();
        }
      } else if (artifacts.some((artifact) => artifact.file === id)) {
        const newArtifacts = await getArtifacts(schema.schema, id);
        if (newArtifacts.some((artifact) => artifact.hash !== artifacts.find((a) => a.file === id)?.hash)) {
          console.log('😼 Artifact change detected');
          artifacts = artifacts.filter((artifact) => artifact.file !== id);
          artifacts.push(...newArtifacts);
          await writeArtifacts();
        }
      } else if (event === 'create' && id.endsWith('.svelte')) {
        console.log('😼 New file detected', id);
        artifacts = await getAllArtifacts(schema.schema, projectDir);
        await writeArtifacts();
      }
    },

    transform(code, id) {
      if (id.endsWith('.svelte')) {
        const scriptMatch = code.match(/<script\s+lang="ts"[^>]*>([\s\S]*?)<\/script>/);
        if (scriptMatch) {
          const scriptContent = scriptMatch[1];
          const transformed = transformGraphQL(artifacts, scriptContent);
          if (transformed) {
            return {
              code: code.replace(scriptMatch[1], transformed),
              map: { mappings: '' },
            };
          }
        }
      }

      const transformed = transformLoad(artifacts, code, id);
      if (!transformed) {
        return;
      }

      return {
        code: transformed,
        map: { mappings: '' },
      };
    },
  };
};
