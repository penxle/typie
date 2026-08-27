import { ToolFailureSchema } from '@typie/prism';
import { ACTION_TAILS } from '../tools/action-cards.ts';
import { toolCallFailureLabels, toolCallLabels } from '../tools/index.ts';

export const labelForRequest = (tool: string, result: unknown): string | null => {
  const label = toolCallLabels[tool];
  if (label === undefined) return null;
  return ToolFailureSchema.safeParse(result).success ? (toolCallFailureLabels[tool] ?? ACTION_TAILS.failed) : label;
};
