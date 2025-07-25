export type SpellingError = {
  id: string;
  from: number;
  to: number;
  context: string;
  corrections: string[];
  explanation: string;
};

export type CheckSpellingResult = {
  from: number;
  to: number;
  context: string;
  corrections: string[];
  explanation: string;
};
