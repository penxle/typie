import { beforeEach, describe, expect, it, vi } from 'vitest';
import { uploadBlob } from './blob.svelte';

const mocks = vi.hoisted(() => ({
  mutation: vi.fn(),
  post: vi.fn(),
}));

vi.mock('$lib/graphql', () => ({
  mearieClient: { mutation: mocks.mutation },
}));

vi.mock('$mearie', () => ({
  graphql: vi.fn(() => ({})),
}));

vi.mock('ky', () => ({
  default: { post: mocks.post },
}));

const files = Array.from({ length: 6 }, (_, index) => new File([`file-${index}`], `file-${index}.txt`, { type: 'text/plain' }));

const prepareTransfers = () => {
  const transfers = files.map(() => Promise.withResolvers<boolean>());
  let transferIndex = 0;
  let active = 0;
  let maxActive = 0;

  mocks.mutation.mockImplementation(async (_document, { input }: { input: { filename: string } }) => ({
    issueBlobUploadUrl: {
      path: `path/${input.filename}`,
      url: `https://uploads.example/${input.filename}`,
      fields: {},
    },
  }));
  mocks.post.mockImplementation(() => {
    const transfer = transfers[transferIndex++];
    active++;
    maxActive = Math.max(maxActive, active);
    return transfer.promise.finally(() => active--);
  });

  return { transfers, maxActive: () => maxActive };
};

describe('blob upload concurrency', () => {
  beforeEach(() => {
    mocks.mutation.mockReset();
    mocks.post.mockReset();
  });

  it('starts the sixth transfer after one of five succeeds', async () => {
    const { transfers, maxActive } = prepareTransfers();
    const uploads = files.map(uploadBlob);

    try {
      await vi.waitFor(() => expect(mocks.post).toHaveBeenCalledTimes(5));
      expect(mocks.mutation).toHaveBeenCalledTimes(5);
      expect(maxActive()).toBe(5);

      transfers[0].resolve(true);
      await vi.waitFor(() => expect(mocks.post).toHaveBeenCalledTimes(6));

      for (const { resolve } of transfers.slice(1)) resolve(true);
      await expect(Promise.all(uploads)).resolves.toHaveLength(6);
      expect(maxActive()).toBe(5);
    } finally {
      for (const { resolve } of transfers) resolve(true);
      await Promise.allSettled(uploads);
    }
  });

  it('starts the sixth transfer after one of five fails', async () => {
    const { transfers, maxActive } = prepareTransfers();
    const uploads = files.map((file) => uploadBlob(file).catch((err: unknown) => err));

    try {
      await vi.waitFor(() => expect(mocks.post).toHaveBeenCalledTimes(5));
      expect(mocks.mutation).toHaveBeenCalledTimes(5);
      expect(maxActive()).toBe(5);

      transfers[0].reject(new Error('transfer failed'));
      await vi.waitFor(() => expect(mocks.post).toHaveBeenCalledTimes(6));

      for (const { resolve } of transfers.slice(1)) resolve(true);
      const results = await Promise.all(uploads);
      expect(results[0]).toBeInstanceOf(Error);
      expect(maxActive()).toBe(5);
    } finally {
      for (const { resolve } of transfers) resolve(true);
      await Promise.allSettled(uploads);
    }
  });
});
