import type { PhasePrompt } from '../../core/contracts.ts';

export type EditorialPhase = 'research' | 'plan' | 'planReview' | 'execute' | 'local' | 'compose' | 'composeReview';
export type EditorialPrompts = Record<EditorialPhase, PhasePrompt>;
