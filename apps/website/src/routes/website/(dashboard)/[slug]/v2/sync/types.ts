export type ClientCommitInput = {
  commitHash: string;
  parentCommitHash: string;
  rootObjectHash: string;
  steps: unknown;
  meta: unknown;
  committedAt: string;
};

export type DocumentObjectInput = {
  hash: string;
  content: unknown;
};

export type OutboxEntry = {
  commit: ClientCommitInput;
  objects: DocumentObjectInput[];
  sequence: number;
};

export type PushStatus = 'idle' | 'pushing' | 'retrying' | 'error';
