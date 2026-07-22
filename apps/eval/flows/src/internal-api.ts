export type Candidate = { documentId: string; text: string; characterCount: number };
export type ExtractResult = { documentId: string; prose: string | null };

export type InternalApi = {
  candidates: (opts?: { limit?: number; minLength?: number; maxLength?: number }) => Promise<Candidate[]>;
  extract: (documentIds: string[]) => Promise<ExtractResult[]>;
};

export const createInternalApi = (base: string, key: string): InternalApi => {
  const post = async <T>(pathname: string, body: unknown): Promise<T> => {
    const response = await fetch(`${base}${pathname}`, {
      method: 'POST',
      headers: { 'content-type': 'application/json', authorization: `Bearer ${key}` },
      body: JSON.stringify(body),
    });
    if (!response.ok) {
      throw new Error(`${pathname} failed: ${response.status}`);
    }
    return (await response.json()) as T;
  };

  return {
    candidates: async (opts = {}) => {
      const { candidates } = await post<{ candidates: Candidate[] }>('/internal/corpus/candidates', opts);
      return candidates;
    },
    extract: async (documentIds) => {
      const { results } = await post<{ results: ExtractResult[] }>('/internal/corpus/extract', { documentIds });
      return results;
    },
  };
};
