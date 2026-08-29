export const josa = (word: string, withFinalConsonant: string, withoutFinalConsonant: string): string => {
  const code = [...word].at(-1)?.codePointAt(0) ?? 0;
  const hasFinalConsonant = code >= 0xac_00 && code <= 0xd7_a3 && (code - 0xac_00) % 28 !== 0;
  return hasFinalConsonant ? withFinalConsonant : withoutFinalConsonant;
};
