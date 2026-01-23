import fs from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { CopyObjectCommand, GetObjectCommand, HeadObjectCommand, PutObjectCommand } from '@aws-sdk/client-s3';
import { createPresignedPost } from '@aws-sdk/s3-presigned-post';
import { getFontMetadata, toWoff2 } from '@typie/fondue';
import { and, eq } from 'drizzle-orm';
import ffmpeg from 'fluent-ffmpeg';
import qs from 'query-string';
import sharp from 'sharp';
import { rgbaToThumbHash } from 'thumbhash';
import { db, Files, first, firstOrThrow, FontFamilies, Fonts, Images, TableCode, validateDbId } from '@/db';
import { FontFamilyState } from '@/enums';
import { stack } from '@/env';
import { TypieError } from '@/errors';
import * as aws from '@/external/aws';
import { builder } from '../builder';
import { Blob, File, Font, Image, isTypeOf } from '../objects';

type VideoMetadata = {
  width: number;
  height: number;
};

function detectAnimatedImage(buffer: Uint8Array): { animated: true; format: 'gif' | 'webp' | 'png' } | null {
  const header = new TextDecoder('ascii').decode(buffer.subarray(0, Math.min(buffer.length, 32_768)));

  if (
    buffer[0] === 0x47 &&
    buffer[1] === 0x49 &&
    buffer[2] === 0x46 &&
    (header.includes('NETSCAPE2.0') || header.includes('NETSCAPE 2.0'))
  ) {
    return { animated: true, format: 'gif' };
  }

  if (buffer[0] === 0x52 && buffer[1] === 0x49 && buffer[2] === 0x46 && buffer[3] === 0x46 && header.includes('ANIM')) {
    return { animated: true, format: 'webp' };
  }

  if (buffer[0] === 0x89 && buffer[1] === 0x50 && buffer[2] === 0x4e && buffer[3] === 0x47 && header.includes('acTL')) {
    return { animated: true, format: 'png' };
  }

  return null;
}

function getVideoMetadata(filePath: string): Promise<VideoMetadata> {
  return new Promise((resolve, reject) => {
    ffmpeg.ffprobe(filePath, (err, metadata) => {
      if (err) return reject(err);
      const videoStream = metadata.streams.find((s) => s.codec_type === 'video');
      if (!videoStream) return reject(new Error('No video stream found'));
      resolve({
        width: videoStream.width ?? 0,
        height: videoStream.height ?? 0,
      });
    });
  });
}

function convertToMp4(inputPath: string, outputPath: string): Promise<void> {
  return new Promise((resolve, reject) => {
    ffmpeg(inputPath)
      .outputOptions([
        '-movflags',
        'faststart',
        '-pix_fmt',
        'yuv420p',
        '-vf',
        'scale=trunc(iw/2)*2:trunc(ih/2)*2',
        '-c:v',
        'libx264',
        '-preset',
        'fast',
        '-crf',
        '23',
      ])
      .noAudio()
      .toFormat('mp4')
      .on('end', () => resolve())
      .on('error', (err) => reject(err))
      .save(outputPath);
  });
}

/**
 * * Types
 */

Blob.implement({
  fields: (t) => ({
    id: t.exposeID('id'),
    size: t.exposeInt('size'),
  }),
});

File.implement({
  isTypeOf: isTypeOf(TableCode.FILES),
  interfaces: [Blob],
  fields: (t) => ({
    name: t.exposeString('name'),

    url: t.string({ resolve: (blob) => `https://typie.net/files/${blob.path}` }),
  }),
});

Image.implement({
  isTypeOf: isTypeOf(TableCode.IMAGES),
  interfaces: [Blob],
  fields: (t) => ({
    placeholder: t.exposeString('placeholder'),
    width: t.exposeInt('width'),
    height: t.exposeInt('height'),

    ratio: t.float({ resolve: (image) => image.width / image.height }),
    url: t.string({
      resolve: (blob) => {
        const prefix = blob.format === 'video/mp4' ? 'videos' : 'images';
        return `https://typie.net/${prefix}/${blob.path}`;
      },
    }),
  }),
});

/**
 * * Queries
 */

builder.queryFields((t) => ({
  image: t.field({
    type: Image,
    args: { imageId: t.arg.id({ validate: validateDbId(TableCode.IMAGES) }) },
    resolve: async (_, args) => {
      return args.imageId;
    },
  }),
}));

/**
 * * Mutations
 */

builder.mutationFields((t) => ({
  issueBlobUploadUrl: t.withAuth({ session: true }).fieldWithInput({
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
          'x-amz-meta-user-id': ctx.session.userId,
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

  persistBlobAsFile: t.withAuth({ session: true }).fieldWithInput({
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
          ContentType: head.ContentType,
          ContentDisposition: `attachment; filename="${fileName}"`,
          MetadataDirective: 'REPLACE',
          TaggingDirective: 'REPLACE',
          Tagging: qs.stringify({
            UserId: ctx.session.userId,
            Environment: stack,
          }),
        }),
      );

      /* eslint-disable @typescript-eslint/no-non-null-assertion */
      return await db
        .insert(Files)
        .values({
          userId: ctx.session.userId,
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

  persistBlobAsImage: t.withAuth({ session: true }).fieldWithInput({
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

      const animatedInfo = detectAnimatedImage(buffer);

      if (animatedInfo) {
        const tempDir = await fs.mkdtemp(path.join(tmpdir(), 'animated-'));

        try {
          const inputPath = path.join(tempDir, `input.${animatedInfo.format}`);
          const outputPath = path.join(tempDir, 'output.mp4');

          await fs.writeFile(inputPath, buffer);
          await convertToMp4(inputPath, outputPath);

          const videoMeta = await getVideoMetadata(outputPath);
          const mp4Buffer = await fs.readFile(outputPath);

          const firstFrame = await sharp(buffer, { pages: 1 })
            .resize({ width: 100, height: 100, fit: 'inside' })
            .ensureAlpha()
            .raw()
            .toBuffer({ resolveWithObject: true });
          const placeholder = rgbaToThumbHash(firstFrame.info.width, firstFrame.info.height, firstFrame.data);

          const basePath = input.path.replace(/\.[^.]+$/, '');
          const mp4Path = `${basePath}.mp4`;

          await aws.s3.send(
            new PutObjectCommand({
              Bucket: 'typie-usercontents',
              Key: `videos/${mp4Path}`,
              Body: mp4Buffer,
              ContentType: 'video/mp4',
              Tagging: qs.stringify({
                UserId: ctx.session.userId,
                Environment: stack,
              }),
            }),
          );

          /* eslint-disable @typescript-eslint/no-non-null-assertion */
          return await db
            .insert(Images)
            .values({
              userId: ctx.session.userId,
              name: decodeURIComponent(object.Metadata!.name),
              size: mp4Buffer.length,
              format: 'video/mp4',
              width: videoMeta.width,
              height: videoMeta.height,
              path: mp4Path,
              placeholder: placeholder.toBase64(),
            })
            .returning()
            .then(firstOrThrow);
          /* eslint-enable @typescript-eslint/no-non-null-assertion */
        } finally {
          // eslint-disable-next-line @typescript-eslint/no-empty-function
          await fs.rm(tempDir, { recursive: true, force: true }).catch(() => {});
        }
      }

      let processed = sharp(buffer, { failOn: 'none', limitInputPixels: false }).rotate();

      if (input.modification) {
        if (input.modification.ensureAlpha) {
          processed = processed.ensureAlpha();
        }

        if (input.modification.resize) {
          processed = processed.resize(input.modification.resize);
        }

        if (input.modification.format) {
          processed = processed.toFormat(input.modification.format);
        }
      }

      const res = await processed.toBuffer({ resolveWithObject: true });
      const data = res.data;
      const info = res.info;

      const mimetype = info.format === 'svg' ? 'image/svg+xml' : `image/${info.format}`;

      const raw = await sharp(data, { pages: 1 })
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
          Tagging: qs.stringify({
            UserId: ctx.session.userId,
            Environment: stack,
          }),
        }),
      );

      /* eslint-disable @typescript-eslint/no-non-null-assertion */
      return await db
        .insert(Images)
        .values({
          userId: ctx.session.userId,
          name: decodeURIComponent(object.Metadata!.name),
          size: data.length,
          format: mimetype,
          width: info.width!,
          height: info.pageHeight || info.height!,
          path: input.path,
          placeholder: placeholder.toBase64(),
        })
        .returning()
        .then(firstOrThrow);
      /* eslint-enable @typescript-eslint/no-non-null-assertion */
    },
  }),

  persistBlobAsFont: t.withAuth({ session: true }).fieldWithInput({
    type: Font,
    input: { path: t.input.string() },
    resolve: async (_, { input }, ctx) => {
      const object = await aws.s3.send(
        new GetObjectCommand({
          Bucket: 'typie-uploads',
          Key: input.path,
        }),
      );

      // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
      const buffer = await object.Body!.transformToByteArray();

      const metadata = getFontMetadata(buffer);

      if (metadata.style !== 'normal') {
        throw new TypieError({ code: 'invalid_font_style' });
      }

      const filePath = path.join(path.dirname(input.path), `${path.basename(input.path, path.extname(input.path))}.woff2`);
      const woff2 = toWoff2(buffer);

      const name =
        metadata.familyName ??
        metadata.fullName ??
        metadata.postScriptName ??
        // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
        path.basename(decodeURIComponent(object.Metadata!.name), path.extname(decodeURIComponent(object.Metadata!.name)));

      await aws.s3.send(
        new PutObjectCommand({
          Bucket: 'typie-usercontents',
          Key: `fonts/${filePath}`,
          Body: woff2,
          ContentType: 'font/woff2',
          Tagging: qs.stringify({
            UserId: ctx.session.userId,
            Environment: stack,
          }),
        }),
      );

      let familyId: string | null = null;
      const familyName = metadata.familyName || name;

      return await db.transaction(async (tx) => {
        const fontFamily = await tx
          .select({ id: FontFamilies.id, state: FontFamilies.state })
          .from(FontFamilies)
          .where(and(eq(FontFamilies.userId, ctx.session.userId), eq(FontFamilies.name, familyName)))
          .then(first);

        if (fontFamily) {
          familyId = fontFamily.id;

          if (fontFamily.state === FontFamilyState.ARCHIVED) {
            await tx.update(FontFamilies).set({ state: FontFamilyState.ACTIVE }).where(eq(FontFamilies.id, familyId));
          }
        } else {
          const fontFamily = await tx
            .insert(FontFamilies)
            .values({
              userId: ctx.session.userId,
              name: familyName,
            })
            .returning({ id: FontFamilies.id })
            .then(firstOrThrow);

          familyId = fontFamily.id;
        }

        const existingFont = await tx
          .select({ id: Fonts.id })
          .from(Fonts)
          .where(and(eq(Fonts.familyId, familyId), eq(Fonts.weight, metadata.weight)))
          .then(first);

        if (existingFont) {
          await tx.delete(Fonts).where(eq(Fonts.id, existingFont.id));
        }

        return await tx
          .insert(Fonts)
          .values({
            familyId,
            name,
            fullName: metadata.fullName,
            postScriptName: metadata.postScriptName,
            weight: metadata.weight,
            size: woff2.length,
            path: filePath,
          })
          .returning()
          .then(firstOrThrow);
      });
    },
  }),
}));
