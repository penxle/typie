import { match } from 'ts-pattern';
import type { Context, ProjectedDeltaFrame, TurnContext } from '@typie/prism';

export type TurnLive = {
  context: TurnContext;
  text: string;
  textBroken: boolean;
  thinkingChars: number;
  toolInput: { name: string } | null;
};

const sameTurn = (a: TurnContext, b: { agent?: { id: string }; run?: number; turn?: number; attempt?: number }): boolean =>
  a.agent.id === b.agent?.id && a.run === b.run && a.turn === b.turn && a.attempt === b.attempt;

export const applyDelta = (current: TurnLive | null, delta: ProjectedDeltaFrame): TurnLive => {
  const base: TurnLive =
    current !== null && sameTurn(current.context, delta.context)
      ? current
      : { context: delta.context, text: '', textBroken: false, thinkingChars: 0, toolInput: null };
  return match(delta)
    .with({ channel: 'thinking' }, ({ chars }) => ({ ...base, thinkingChars: chars }))
    .with({ channel: 'tool.input' }, ({ tool }) => ({ ...base, toolInput: { name: tool.name } }))
    .with({ channel: 'text' }, ({ offset, data }) => {
      if (offset === 0) return { ...base, text: data, textBroken: false };
      if (base.textBroken) return base;
      if (offset !== base.text.length) return { ...base, textBroken: true };
      return { ...base, text: base.text + data };
    })
    .exhaustive();
};

export const sealTurn = (current: TurnLive | null, context: Context): TurnLive | null => {
  if (current === null) return null;
  if (context.agent !== undefined && context.agent.id !== current.context.agent.id) return current;
  if (context.turn !== undefined && context.turn !== current.context.turn) return current;
  return null;
};

export const startTurn = (current: TurnLive | null, context: Context): TurnLive | null => {
  if (current === null) return null;
  return sameTurn(current.context, context) ? current : null;
};
