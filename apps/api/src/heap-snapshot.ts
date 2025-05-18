import os from 'node:os';
import { CompleteMultipartUploadCommand, CreateMultipartUploadCommand, UploadPartCommand } from '@aws-sdk/client-s3';
import { logger } from '@typie/lib';
import dayjs from 'dayjs';
import * as aws from '@/external/aws';
import { dev } from './env';
import type { UploadPartCommandOutput } from '@aws-sdk/client-s3';

const takeSnapshot = async () => {
  try {
    logger.warn('Taking heap snapshot...');
    const snapshot = new TextEncoder().encode(Bun.generateHeapSnapshot('v8'));
    logger.warn('Heap snapshot taken', { snapshotLength: snapshot.byteLength });

    const hostname = os.hostname();
    const datetime = dayjs.utc().format('YYYY-MM-DD_HH-mm-ss');
    const bucket = 'typie-misc';
    const key = `heap-snapshots/${hostname}-${datetime}.heapsnapshot`;

    const upload = await aws.s3.send(
      new CreateMultipartUploadCommand({
        Bucket: bucket,
        Key: key,
        ContentType: 'application/json',
      }),
    );

    const chunkSize = 5 * 1024 * 1024;
    const promises: Promise<UploadPartCommandOutput>[] = [];

    let partNumber = 1;
    let offset = 0;

    while (offset < snapshot.byteLength) {
      const chunkEnd = Math.min(offset + chunkSize, snapshot.byteLength);
      const chunk = snapshot.subarray(offset, chunkEnd);

      promises.push(
        aws.s3.send(
          new UploadPartCommand({
            Bucket: bucket,
            Key: key,
            UploadId: upload.UploadId,
            PartNumber: partNumber,
            Body: chunk,
          }),
        ),
      );

      partNumber++;
      offset += chunkSize;
    }

    const results = await Promise.all(promises);

    if (results.length > 0) {
      await aws.s3.send(
        new CompleteMultipartUploadCommand({
          Bucket: bucket,
          Key: key,
          UploadId: upload.UploadId,
          MultipartUpload: {
            Parts: results.map((result, index) => ({ ETag: result.ETag, PartNumber: index + 1 })),
          },
        }),
      );
    }

    logger.warn('Heap snapshot uploaded successfully.', { key });
  } catch (err) {
    logger.error(err, 'Error during heap snapshot processing or upload.');
  }
};

if (!dev) {
  setInterval(takeSnapshot, 60 * 1000);
  takeSnapshot();
}
