export type CompositionTailState =
  | { type: 'idle'; generation: number }
  | { type: 'key_observed'; generation: number; key: string }
  | { type: 'edit_deferred'; generation: number; appendedText: string };

export type CompositionTailObservation =
  | { type: 'key_down'; key: string | null }
  | { type: 'composition_edit'; currentText: string | null; editText: string; targetsCurrentComposition: boolean }
  | { type: 'composition_continues' }
  | { type: 'composition_end' }
  | { type: 'timeout'; generation: number }
  | { type: 'reset' };

export type CompositionTailEffect =
  | { type: 'apply_current_edit' }
  | { type: 'defer_current_edit'; generation: number }
  | { type: 'apply_deferred_edit'; generation: number }
  | { type: 'commit_then_insert'; generation: number; text: string }
  | { type: 'discard_deferred_edit'; generation: number };

export type CompositionTailResolution = {
  state: CompositionTailState;
  effects: CompositionTailEffect[];
};

export const initialCompositionTailState: CompositionTailState = { type: 'idle', generation: 0 };

const idle = (generation: number): CompositionTailState => ({ type: 'idle', generation });

export const resolveCompositionTail = (state: CompositionTailState, observation: CompositionTailObservation): CompositionTailResolution => {
  switch (observation.type) {
    case 'key_down': {
      const effects: CompositionTailEffect[] =
        state.type === 'edit_deferred' ? [{ type: 'apply_deferred_edit', generation: state.generation }] : [];
      return {
        state:
          observation.key == null ? idle(state.generation) : { type: 'key_observed', generation: state.generation, key: observation.key },
        effects,
      };
    }
    case 'composition_edit': {
      if (
        state.type === 'key_observed' &&
        observation.currentText != null &&
        observation.targetsCurrentComposition &&
        observation.editText === `${observation.currentText}${state.key}`
      ) {
        const generation = state.generation + 1;
        return {
          state: { type: 'edit_deferred', generation, appendedText: state.key },
          effects: [{ type: 'defer_current_edit', generation }],
        };
      }

      const effects: CompositionTailEffect[] = [];
      if (state.type === 'edit_deferred') {
        effects.push({ type: 'apply_deferred_edit', generation: state.generation });
      }
      effects.push({ type: 'apply_current_edit' });
      return { state: idle(state.generation), effects };
    }
    case 'composition_continues': {
      return state.type === 'edit_deferred'
        ? { state: idle(state.generation), effects: [{ type: 'apply_deferred_edit', generation: state.generation }] }
        : { state, effects: [] };
    }
    case 'composition_end': {
      return state.type === 'edit_deferred'
        ? {
            state: idle(state.generation),
            effects: [{ type: 'commit_then_insert', generation: state.generation, text: state.appendedText }],
          }
        : { state: idle(state.generation), effects: [] };
    }
    case 'timeout': {
      return state.type === 'edit_deferred' && state.generation === observation.generation
        ? { state: idle(state.generation), effects: [{ type: 'apply_deferred_edit', generation: state.generation }] }
        : { state, effects: [] };
    }
    case 'reset': {
      const effects: CompositionTailEffect[] =
        state.type === 'edit_deferred' ? [{ type: 'discard_deferred_edit', generation: state.generation }] : [];
      return { state: idle(state.generation + 1), effects };
    }
  }
};
