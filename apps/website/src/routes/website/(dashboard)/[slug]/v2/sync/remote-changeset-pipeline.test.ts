import { expect, it, vi } from 'vitest';
import { RemoteChangesetPipeline } from './remote-changeset-pipeline';

const event = (seq: string, value: number) => ({
  seq,
  bundles: [new Uint8Array([value])],
  heads: new Uint8Array([value]),
  durableHeads: new Uint8Array([value]),
});

it('commits remote changeset metadata in event order when editor updates settle out of order', async () => {
  const first = Promise.withResolvers<number>();
  const second = Promise.withResolvers<number>();
  const completions = [first, second];
  const committed: string[] = [];
  const editor = {
    receiveRemoteChangeset: vi.fn(() => {
      const completion = completions.shift();
      if (!completion) throw new Error('Unexpected remote changeset');
      return completion.promise;
    }),
  };
  const pipeline = new RemoteChangesetPipeline(editor, ({ seq }) => {
    committed.push(seq);
  });

  const firstApplied = pipeline.apply(event('1', 1));
  const secondApplied = pipeline.apply(event('2', 2));

  expect(editor.receiveRemoteChangeset).toHaveBeenCalledTimes(2);
  second.resolve(2);
  await Promise.resolve();
  expect(committed).toEqual([]);

  first.resolve(1);
  await Promise.all([firstApplied, secondApplied]);
  expect(committed).toEqual(['1', '2']);
});
