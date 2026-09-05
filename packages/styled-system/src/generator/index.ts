import path from 'node:path';
import { CONTRAST_GATE, failingPairs } from './contrast.ts';
import { emitConditions, emitContrastReport, emitNotices, emitPresets, emitSemanticColors, emitThemeJson, formatOutput } from './emit.ts';
import { loadThemes } from './load.ts';

export const REPO_ROOT = path.resolve(import.meta.dirname, '../../../..');

export const OUTPUT_PATHS = {
  semanticColors: 'packages/styled-system/src/semantic-colors.generated.ts',
  conditions: 'packages/styled-system/src/conditions.generated.ts',
  presets: 'packages/styled-system/src/presets.generated.ts',
  themeJson: 'assets/theme.json',
  contrastReport: 'assets/themes/contrast-report.md',
  notices: 'assets/themes/NOTICES.md',
} as const;

export type Generated = { outputs: Record<string, string>; failures: string[] };

export const generate = async (repoRoot: string = REPO_ROOT): Promise<Generated> => {
  const source = loadThemes(path.join(repoRoot, 'assets/themes'));
  const raw: Record<string, string> = {
    [OUTPUT_PATHS.semanticColors]: emitSemanticColors(source),
    [OUTPUT_PATHS.conditions]: emitConditions(source),
    [OUTPUT_PATHS.presets]: emitPresets(source),
    [OUTPUT_PATHS.themeJson]: emitThemeJson(source),
    [OUTPUT_PATHS.contrastReport]: emitContrastReport(source),
    [OUTPUT_PATHS.notices]: emitNotices(source),
  };
  const outputs: Record<string, string> = {};
  for (const [relative, content] of Object.entries(raw)) {
    outputs[relative] = await formatOutput(content, path.join(repoRoot, relative));
  }
  return { outputs, failures: CONTRAST_GATE ? failingPairs(source.presets) : [] };
};
