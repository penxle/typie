import * as aws from '@pulumi/aws';
import * as pulumi from '@pulumi/pulumi';

const cdn = new aws.s3.Bucket('cdn', {
  bucket: 'typie-cdn',
});

new aws.s3.BucketPolicy('cdn', {
  bucket: cdn.bucket,
  policy: {
    Version: '2012-10-17',
    Statement: [
      {
        Effect: 'Allow',
        Principal: { Service: 'cloudfront.amazonaws.com' },
        Action: ['s3:GetObject'],
        Resource: [pulumi.interpolate`${cdn.arn}/*`],
      },
    ],
  },
});

const usercontents = new aws.s3.Bucket('usercontents', {
  bucket: 'typie-usercontents',
});

new aws.s3.BucketPolicy('usercontents', {
  bucket: usercontents.bucket,
  policy: {
    Version: '2012-10-17',
    Statement: [
      {
        Effect: 'Allow',
        Principal: { Service: 'cloudfront.amazonaws.com' },
        Action: ['s3:GetObject'],
        Resource: [pulumi.interpolate`${usercontents.arn}/*`],
      },
    ],
  },
});

new aws.s3.BucketLifecycleConfiguration('usercontents', {
  bucket: usercontents.bucket,
  rules: [
    {
      id: 'transition-to-intelligent-tiering',
      status: 'Enabled',
      transitions: [
        {
          days: 0,
          storageClass: 'INTELLIGENT_TIERING',
        },
      ],
    },
  ],
});

const uploads = new aws.s3.Bucket('uploads', {
  bucket: 'typie-uploads',
});

new aws.s3.BucketLifecycleConfiguration('uploads', {
  bucket: uploads.bucket,
  rules: [
    {
      id: 'delete-after-1-day',
      status: 'Enabled',
      expiration: {
        days: 1,
      },
    },
  ],
});

new aws.s3.BucketCorsConfiguration('uploads', {
  bucket: uploads.bucket,
  corsRules: [
    {
      allowedHeaders: ['*'],
      allowedMethods: ['POST'],
      allowedOrigins: ['https://typie.co', 'https://typie.dev', 'http://localhost:4100'],
    },
  ],
});

const misc = new aws.s3.Bucket('misc', {
  bucket: 'typie-misc',
});

export const buckets = { cdn, usercontents, uploads, misc };

export const outputs = {
  AWS_S3_BUCKET_USERCONTENTS_ARN: usercontents.arn,
  AWS_S3_BUCKET_UPLOADS_ARN: uploads.arn,
  AWS_S3_BUCKET_MISC_ARN: misc.arn,
};
