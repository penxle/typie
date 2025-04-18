import { init } from '@paralleldrive/cuid2';
import { match } from 'ts-pattern';

const createSid = init({ length: 10 });
const createMid = init({ length: 14 });
const createLid = init({ length: 18 });

export const createDbId = (tableCode: string, opt?: { length: 'short' | 'moderate' | 'long' }) => {
  const fn = match(opt?.length ?? 'moderate')
    .with('short', () => createSid)
    .with('moderate', () => createMid)
    .with('long', () => createLid)
    .exhaustive();

  return `${tableCode}0${fn().toUpperCase()}`;
};

export const decodeDbId = (id: string) => {
  return id.split('0', 2)[0];
};

export const validateDbId = (tableCode: string) => {
  return { regex: new RegExp(String.raw`^${tableCode}0[A-Z0-9]+$`) };
};
