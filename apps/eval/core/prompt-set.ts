import type { GenerationManifest, PhasePrompt, PhaseSpec } from './contracts.ts';

export const promptPhases = (manifest: GenerationManifest): PhaseSpec[] => manifest.phases.filter((p) => p.prompt !== false);

const isPrompt = (value: unknown): value is PhasePrompt => {
  if (!value || typeof value !== 'object') return false;
  const prompt = value as Record<string, unknown>;
  return typeof prompt.system === 'string' && typeof prompt.model === 'string';
};

export const validatePromptSet = (manifest: GenerationManifest, content: Record<string, unknown>): string[] => {
  const allowed = new Set(promptPhases(manifest).map((p) => p.key));
  const violations: string[] = [];

  for (const phase of promptPhases(manifest)) {
    const value = content[phase.key];
    if (value === undefined) {
      violations.push(`${phase.key} 단계의 프롬프트가 없습니다`);
      continue;
    }
    if (!isPrompt(value)) {
      const prompt = (value ?? {}) as Record<string, unknown>;
      violations.push(typeof prompt.system === 'string' ? `${phase.key} 단계에 model이 없습니다` : `${phase.key} 단계에 system이 없습니다`);
    }
  }

  for (const key of Object.keys(content)) {
    if (!allowed.has(key)) violations.push(`${key}는 이 세대에 없는 단계입니다`);
  }

  return violations;
};

export const resolvePrompts = (manifest: GenerationManifest, content: Record<string, unknown>): Record<string, PhasePrompt> => {
  const violations = validatePromptSet(manifest, content);
  if (violations.length > 0) throw new Error(violations.join(' / '));
  return Object.fromEntries(promptPhases(manifest).map((p) => [p.key, content[p.key] as PhasePrompt]));
};
