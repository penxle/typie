export type FindReplaceResult = {
  from: number;
  to: number;
  index: number;
};

export type FindReplaceState = {
  searchText: string;
  replaceText: string;
  currentMatch: number;
  totalMatches: number;
  results: FindReplaceResult[];
};
