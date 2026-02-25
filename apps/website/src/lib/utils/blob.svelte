<script lang="ts" module>
  import ky from 'ky';
  import { mearieClient } from '$lib/graphql';
  import { graphql } from '$mearie';

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
        size
        width
        height
        ratio
        placeholder
      }
    }
  `);

  export const uploadBlob = async (file: File) => {
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
  };

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
</script>
