import { generateJitteredKeyBetween, indexCharacterSet } from 'fractional-indexing-jittered';

const charSet = indexCharacterSet({ chars: 'ABCDEFGHIJKLMNOPQRSTUVWXYZ' });
type GenerateFractionalOrderParams = { lower: string | null | undefined; upper: string | null | undefined };
export const generateFractionalOrder = ({ lower, upper }: GenerateFractionalOrderParams) => {
  return generateJitteredKeyBetween(lower ?? null, upper ?? null, charSet);
};
