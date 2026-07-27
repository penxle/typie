<script lang="ts" module>
  import ky from 'ky';
  import pLimit from 'p-limit';
  import { mearieClient } from '$lib/graphql';
  import { graphql } from '$mearie';

  const MAX_CONCURRENT_BLOB_UPLOADS = 5;
  const limitBlobUploads = pLimit(MAX_CONCURRENT_BLOB_UPLOADS);

  const issueBlobUploadUrlMutation = graphql(`
    mutation BlobUtils_IssueBlobUploadUrl($input: IssueBlobUploadUrlInput!) {
      issueBlobUploadUrl(input: $input) {
        path
        url
        fields
      }
    }
  `);

  const persistBlobAsFileMutation = graphql(`
    mutation BlobUtils_PersistBlobAsFile($input: PersistBlobAsFileInput!) {
      persistBlobAsFile(input: $input) {
        id
        name
        size
        url
      }
    }
  `);

  const persistBlobAsImageMutation = graphql(`
    mutation BlobUtils_PersistBlobAsImage($input: PersistBlobAsImageInput!) {
      persistBlobAsImage(input: $input) {
        id
        url
        originalUrl
        size
        width
        height
        ratio
        placeholder
      }
    }
  `);

  export const uploadBlob = (file: File) =>
    limitBlobUploads(async () => {
      const {
        issueBlobUploadUrl: { path, url, fields },
      } = await mearieClient.mutation(issueBlobUploadUrlMutation, { input: { filename: file.name } });

      const formData = new FormData();
      for (const [key, value] of Object.entries<string>(fields)) {
        formData.append(key, value);
      }

      formData.append('Content-Type', file.type);
      formData.append('file', file);

      await ky.post(url, {
        body: formData,
        timeout: false,
      });

      return path;
    });

  export const uploadBlobAsFile = async (file: File) => {
    const path = await uploadBlob(file);
    const result = await mearieClient.mutation(persistBlobAsFileMutation, { input: { path } });
    return result.persistBlobAsFile;
  };

  export const uploadBlobAsImage = async (file: File, modification?: unknown) => {
    const path = await uploadBlob(file);
    const result = await mearieClient.mutation(persistBlobAsImageMutation, { input: { path, modification } });
    return result.persistBlobAsImage;
  };

  const reserveBlobUploadsMutation = graphql(`
    mutation BlobUtils_ReserveBlobUploads($input: ReserveBlobUploadsInput!) {
      reserveBlobUploads(input: $input) {
        assetId
        nonce
        path
        uploadUrl
        uploadFields
      }
    }
  `);

  const finalizeBlobUploadAsImageMutation = graphql(`
    mutation BlobUtils_FinalizeBlobUploadAsImage($input: FinalizeBlobUploadAsImageInput!) {
      finalizeBlobUploadAsImage(input: $input) {
        id
        url
        originalUrl
        width
        height
        placeholder
      }
    }
  `);

  const finalizeBlobUploadAsFileMutation = graphql(`
    mutation BlobUtils_FinalizeBlobUploadAsFile($input: FinalizeBlobUploadAsFileInput!) {
      finalizeBlobUploadAsFile(input: $input) {
        id
        name
        size
        url
      }
    }
  `);

  const abandonBlobUploadMutation = graphql(`
    mutation BlobUtils_AbandonBlobUpload($input: AbandonBlobUploadInput!) {
      abandonBlobUpload(input: $input)
    }
  `);

  export type ReserveBlobUploadItem = {
    kind: 'image' | 'file';
    name: string;
    format: string;
    size: number;
    assetId?: string;
  };

  export type BlobUploadReservation = {
    assetId: string;
    nonce: string;
    path: string;
    uploadUrl: string;
    uploadFields: Record<string, string>;
  };

  // 서버(`blob-upload.ts`)와 같은 규칙. presign은 이 값을 Content-Type 조건으로 **정확히 일치** 요구하므로
  // 예약·POST·lease 셋이 모두 같은 값을 써야 한다.
  export const normalizeContentType = (contentType: string | null | undefined): string => contentType?.trim() || 'application/octet-stream';

  export const reserveBlobUploads = async (
    documentId: string,
    items: readonly ReserveBlobUploadItem[],
  ): Promise<BlobUploadReservation[]> => {
    const result = await mearieClient.mutation(reserveBlobUploadsMutation, {
      input: {
        documentId,
        items: items.map((item) => ({
          assetId: item.assetId,
          kind: item.kind === 'image' ? ('IMAGE' as const) : ('FILE' as const),
          name: item.name,
          format: normalizeContentType(item.format),
          size: String(item.size),
        })),
      },
    });

    return result.reserveBlobUploads.map((reservation) => ({
      assetId: reservation.assetId,
      nonce: reservation.nonce,
      path: reservation.path,
      uploadUrl: reservation.uploadUrl,
      uploadFields: reservation.uploadFields as Record<string, string>,
    }));
  };

  // 진행률 콜백 없음(불확정 인디케이터). Content-Type은 서버가 정규화해 `uploadFields`에 실어 준 값을 그대로 쓴다.
  export const uploadToReservation = async (reservation: BlobUploadReservation, file: File, signal?: AbortSignal): Promise<void> => {
    const formData = new FormData();
    for (const [key, value] of Object.entries(reservation.uploadFields)) {
      formData.append(key, value);
    }
    if (!('Content-Type' in reservation.uploadFields)) {
      formData.append('Content-Type', normalizeContentType(file.type));
    }
    formData.append('file', file);

    await ky.post(reservation.uploadUrl, {
      body: formData,
      timeout: false,
      signal,
    });
  };

  export const finalizeBlobUploadAsImage = async (assetId: string, nonce: string) => {
    const result = await mearieClient.mutation(finalizeBlobUploadAsImageMutation, { input: { assetId, nonce } });
    return result.finalizeBlobUploadAsImage;
  };

  export const finalizeBlobUploadAsFile = async (assetId: string, nonce: string) => {
    const result = await mearieClient.mutation(finalizeBlobUploadAsFileMutation, { input: { assetId, nonce } });
    return result.finalizeBlobUploadAsFile;
  };

  export const abandonBlobUpload = async (assetId: string, nonce: string): Promise<boolean> => {
    const result = await mearieClient.mutation(abandonBlobUploadMutation, { input: { assetId, nonce } });
    return result.abandonBlobUpload;
  };
</script>
