import path from 'node:path';
import { CopyObjectCommand, GetObjectCommand, HeadObjectCommand, PutObjectCommand } from '@aws-sdk/client-s3';
import { createPresignedPost } from '@aws-sdk/s3-presigned-post';
import { base64 } from 'rfc4648';
import sharp from 'sharp';
import { rgbaToThumbHash } from 'thumbhash';
import { db, Files, firstOrThrow, Images, TableCode } from '@/db';
import * as aws from '@/external/aws';
import { builder } from '../builder';
import { Blob, File, Image, isTypeOf } from '../objects';

/**
 * * Types
 */

Blob.implement({
  fields: (t) => ({
    id: t.exposeID('id'),
  }),
});

File.implement({
  isTypeOf: isTypeOf(TableCode.FILES),
  interfaces: [Blob],
  fields: (t) => ({
    name: t.exposeString('name'),
    size: t.exposeInt('size'),

    url: t.string({ resolve: (blob) => `https://typie.net/files/${blob.path}` }),
  }),
});

Image.implement({
  isTypeOf: isTypeOf(TableCode.IMAGES),
  interfaces: [Blob],
  fields: (t) => ({
    placeholder: t.exposeString('placeholder'),

    ratio: t.float({ resolve: (image) => image.width / image.height }),
    url: t.string({ resolve: (blob) => `https://typie.net/images/${blob.path}` }),
  }),
});

/**
 * * Mutations
 */

builder.mutationFields((t) => ({
  issueBlobUploadUrl: t.fieldWithInput({
    type: t.builder.simpleObject('IssueBlobUploadUrlResult', {
      fields: (t) => ({
        path: t.string(),
        url: t.string(),
        fields: t.field({ type: 'JSON' }),
      }),
    }),
    input: { filename: t.input.string() },
    resolve: async (_, { input }, ctx) => {
      const ext = path.extname(input.filename);
      const key = `${aws.createFragmentedS3ObjectKey()}${ext}`;

      const req = await createPresignedPost(aws.s3, {
        Bucket: 'typie-uploads',
        Key: key,
        Conditions: [
          ['content-length-range', 0, 1024 * 1024 * 1024], // 1GB
          ['starts-with', '$Content-Type', ''],
        ],
        Fields: {
          'x-amz-meta-name': encodeURIComponent(input.filename),
          'x-amz-meta-user-id': ctx.session?.userId ?? '',
        },
        Expires: 60 * 5, // 5 minutes
      });

      return {
        path: key,
        url: req.url,
        fields: req.fields,
      };
    },
  }),

  persistBlobAsFile: t.fieldWithInput({
    type: File,
    input: { path: t.input.string() },
    resolve: async (_, { input }, ctx) => {
      const head = await aws.s3.send(
        new HeadObjectCommand({
          Bucket: 'typie-uploads',
          Key: input.path,
        }),
      );

      // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
      const fileName = head.Metadata!.name;

      await aws.s3.send(
        new CopyObjectCommand({
          Bucket: 'typie-usercontents',
          Key: `files/${input.path}`,
          CopySource: `typie-uploads/${input.path}`,
          MetadataDirective: 'REPLACE',
          ContentType: head.ContentType,
          ContentDisposition: `attachment; filename="${fileName}"`,
          Metadata: {
            name: fileName,
            'user-id': ctx.session?.userId ?? '',
          },
        }),
      );

      /* eslint-disable @typescript-eslint/no-non-null-assertion */
      return await db
        .insert(Files)
        .values({
          userId: ctx.session?.userId,
          name: decodeURIComponent(fileName),
          size: head.ContentLength!,
          format: head.ContentType ?? 'application/octet-stream',
          path: input.path,
        })
        .returning()
        .then(firstOrThrow);
      /* eslint-enable @typescript-eslint/no-non-null-assertion */
    },
  }),

  persistBlobAsImage: t.fieldWithInput({
    type: Image,
    input: {
      path: t.input.string(),
      modification: t.input.field({ type: 'JSON', required: false }),
    },
    resolve: async (_, { input }, ctx) => {
      const object = await aws.s3.send(
        new GetObjectCommand({
          Bucket: 'typie-uploads',
          Key: input.path,
        }),
      );

      // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
      const buffer = await object.Body!.transformToByteArray();

      let img = sharp(buffer, { failOn: 'none' });

      if (input.modification) {
        if (input.modification.ensureAlpha) {
          img = img.ensureAlpha();
        }

        if (input.modification.resize) {
          img = img.resize(input.modification.resize);
        }

        if (input.modification.format) {
          img = img.toFormat(input.modification.format);
        }
      }

      let data;
      let info;

      if (input.modification) {
        const res = await img.toBuffer({ resolveWithObject: true });
        data = res.data;
        info = res.info;
      } else {
        const res = await img.metadata();
        data = buffer;
        info = res;
      }

      const mimetype = info.format === 'svg' ? 'image/svg+xml' : `image/${info.format}`;

      const raw = await img
        .clone()
        .resize({ width: 100, height: 100, fit: 'inside' })
        .ensureAlpha()
        .raw()
        .toBuffer({ resolveWithObject: true });
      const placeholder = rgbaToThumbHash(raw.info.width, raw.info.height, raw.data);

      await aws.s3.send(
        new PutObjectCommand({
          Bucket: 'typie-usercontents',
          Key: `images/${input.path}`,
          Body: data,
          ContentType: mimetype,
          Metadata: {
            // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
            name: object.Metadata!.name,
            'user-id': ctx.session?.userId ?? '',
          },
        }),
      );

      /* eslint-disable @typescript-eslint/no-non-null-assertion */
      return await db
        .insert(Images)
        .values({
          userId: ctx.session?.userId,
          name: decodeURIComponent(object.Metadata!.name),
          size: data.length,
          format: mimetype,
          width: info.width!,
          height: info.height!,
          path: input.path,
          placeholder: base64.stringify(placeholder),
        })
        .returning()
        .then(firstOrThrow);
      /* eslint-enable @typescript-eslint/no-non-null-assertion */
    },
  }),
}));
