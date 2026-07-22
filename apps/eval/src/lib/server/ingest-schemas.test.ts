import { describe, expect, it } from 'vitest';
import { corpusPayloadSchema, runPayloadSchema } from './ingest-schemas.ts';

describe('corpusPayloadSchema', () => {
  it('유효한 payload를 통과시킨다', () => {
    const payload = {
      corpusVersion: 'v1',
      documents: [{ id: 'd1', refId: 'P0XXXX', content: '본문', characterCount: 2 }],
    };
    expect(corpusPayloadSchema.parse(payload)).toEqual(payload);
  });

  it('빈 documents는 거부한다', () => {
    expect(() => corpusPayloadSchema.parse({ corpusVersion: 'v1', documents: [] })).toThrow();
  });
});

describe('runPayloadSchema', () => {
  it('유효한 payload를 통과시킨다', () => {
    const payload = {
      runId: 'run-1',
      variantLabel: 'V0',
      round: 'analyze-1',
      corpusVersion: 'v1',
      sets: [
        {
          documentId: 'd1',
          feedbacks: [{ startText: 's', endText: 'e', matchStart: 0, matchEnd: 5, category: '가독성', body: '피드백' }],
        },
      ],
    };
    expect(runPayloadSchema.parse(payload)).toMatchObject(payload);
  });

  it('matchStart는 null 허용', () => {
    const payload = {
      runId: 'run-1',
      variantLabel: 'V0',
      round: 'analyze-1',
      corpusVersion: 'v1',
      sets: [
        { documentId: 'd1', feedbacks: [{ startText: 's', endText: 'e', matchStart: null, matchEnd: null, category: null, body: 'b' }] },
      ],
    };
    expect(() => runPayloadSchema.parse(payload)).not.toThrow();
  });
});
