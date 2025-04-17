import path from 'node:path';
import { getAllArtifacts, getArtifacts, getSchema } from '../artifact';
import { writeArtifactAssets, writeMiscAssets, writePublicAssets, writeTypeAssets } from '../codegen/writer';
import { transformGraphQL, transformLoad } from './transform';
import type * as graphql from 'graphql';
import type { PreprocessorGroup } from 'svelte/compiler';
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

  let schema: graphql.GraphQLSchema;
  let artifacts: Artifact[] = [];

  const writeArtifacts = async () => {
    await writeArtifactAssets(resolvedOutDir, schema, artifacts);
    await writePublicAssets(resolvedOutDir, artifacts);
    await writeTypeAssets(resolvedOutDir, projectDir, artifacts);
    await writeMiscAssets(resolvedOutDir);
  };

  const sveltePreprocess: PreprocessorGroup = {
    name: '@typie/sark',
    script: ({ content, attributes }) => {
      if (attributes.lang !== 'ts') {
        return;
      }

      const transformed = transformGraphQL(artifacts, content);
      if (!transformed) {
        return;
      }

      return {
        code: transformed,
        map: { mappings: '' },
      };
    },
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
      artifacts = await getAllArtifacts(schema, projectDir);
      await writeArtifacts();
    },

    async watchChange(id) {
      if (id === resolvedSchemaPath) {
        console.log('😼 Schema changed');
        schema = await getSchema(resolvedSchemaPath);
        artifacts = await getAllArtifacts(schema, projectDir);
        await writeArtifacts();
      } else if (artifacts.some((artifact) => artifact.file === id)) {
        console.log('😼 Artifact changed');
        const newArtifacts = await getArtifacts(schema, id);
        artifacts = artifacts.filter((artifact) => artifact.file !== id);
        artifacts.push(...newArtifacts);
        await writeArtifacts();
      }
    },

    transform(code, id) {
      const transformed = transformLoad(artifacts, code, id);
      if (!transformed) {
        return;
      }

      return {
        code: transformed,
        map: { mappings: '' },
      };
    },

    api: { sveltePreprocess },
  };
};
