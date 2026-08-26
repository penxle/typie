export type PostCommitEffect = () => void | Promise<void>;
export type PostCommitRegistrar = (effect: PostCommitEffect) => void;

export const runPostCommitEffects = async (effects: readonly PostCommitEffect[]): Promise<unknown[]> => {
  const errors: unknown[] = [];
  for (const effect of effects) {
    try {
      await effect();
    } catch (err) {
      errors.push(err);
    }
  }

  return errors;
};

export const runAfterCommit = async (afterCommit: PostCommitRegistrar | undefined, effect: PostCommitEffect): Promise<void> => {
  if (afterCommit) {
    afterCommit(effect);
    return;
  }

  await effect();
};
