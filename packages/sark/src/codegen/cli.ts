import path from 'node:path';
import { getAllArtifacts, getSchema } from './artifact';
import { writeArtifactAssets, writeMiscAssets, writePublicAssets, writeTypeAssets } from './codegen/writer';

const main = async () => {
  const { schemaPath = 'schema.graphql', outDir = '.sark' } = {};

  const projectDir = process.cwd();
  const resolvedOutDir = path.join(projectDir, outDir);
  const resolvedSchemaPath = path.join(projectDir, schemaPath);

  const schema = await getSchema(resolvedSchemaPath);
  const artifacts = await getAllArtifacts(schema, projectDir);

  await writeArtifactAssets(resolvedOutDir, schema, artifacts);
  await writePublicAssets(resolvedOutDir, artifacts);
  await writeMiscAssets(resolvedOutDir);
  await writeTypeAssets(resolvedOutDir, projectDir, artifacts);

  return 0;
};

// eslint-disable-next-line import/no-default-export
export default main;
