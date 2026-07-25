import { beforeEach, describe, expect, it, vi } from 'vitest';
import { normalizeContentType, reserveBlobUploads, uploadBlob, uploadToReservation } from './blob.svelte';

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

describe('reserved blob uploads', () => {
  beforeEach(() => {
    mocks.mutation.mockReset();
    mocks.post.mockReset();
  });

  it.each([
    ['', 'application/octet-stream'],
    [' '.repeat(3), 'application/octet-stream'],
    ['image/png', 'image/png'],
  ])('normalizes %o to %o', (input, expected) => {
    expect(normalizeContentType(input)).toBe(expected);
    expect(normalizeContentType(null)).toBe('application/octet-stream');
    expect(normalizeContentType(undefined)).toBe('application/octet-stream');
  });

  it('sends the normalized content type and a string size in the reservation input', async () => {
    mocks.mutation.mockResolvedValue({ reserveBlobUploads: [] });

    await reserveBlobUploads('DOC1', [
      { kind: 'image', name: 'a.png', format: 'image/png', size: 10 },
      { kind: 'file', name: 'b.bin', format: '', size: 20, assetId: 'FILE1' },
    ]);

    expect(mocks.mutation.mock.calls[0]?.[1]).toEqual({
      input: {
        documentId: 'DOC1',
        items: [
          { assetId: undefined, kind: 'IMAGE', name: 'a.png', format: 'image/png', size: '10' },
          { assetId: 'FILE1', kind: 'FILE', name: 'b.bin', format: 'application/octet-stream', size: '20' },
        ],
      },
    });
  });

  it('posts exactly the presigned fields so the exact-match Content-Type condition holds', async () => {
    mocks.post.mockResolvedValue(undefined);
    const uploaded = new File(['x'], 'x.bin', { type: 'application/pdf' });

    await uploadToReservation(
      {
        assetId: 'FILE1',
        nonce: 'n1',
        path: 'p',
        uploadUrl: 'https://uploads.example/bucket',
        uploadFields: { key: 'p', 'Content-Type': 'application/octet-stream' },
      },
      uploaded,
    );

    const body = mocks.post.mock.calls[0]?.[1]?.body as FormData;
    // 서버 presign은 정규화된 값과의 **정확 일치**를 요구한다 — file.type을 그대로 덧붙이면 유효한 파일이 거부된다.
    expect(body.getAll('Content-Type')).toEqual(['application/octet-stream']);
    expect(body.get('key')).toBe('p');
    expect(body.get('file')).toBe(uploaded);
    expect(mocks.post.mock.calls[0]?.[0]).toBe('https://uploads.example/bucket');
  });

  it('falls back to the normalized file type when the presign carries no content type field', async () => {
    mocks.post.mockResolvedValue(undefined);

    await uploadToReservation(
      { assetId: 'FILE1', nonce: 'n1', path: 'p', uploadUrl: 'https://uploads.example', uploadFields: { key: 'p' } },
      new File(['x'], 'x.bin', { type: '' }),
    );

    const body = mocks.post.mock.calls[0]?.[1]?.body as FormData;
    expect(body.getAll('Content-Type')).toEqual(['application/octet-stream']);
  });
});

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
