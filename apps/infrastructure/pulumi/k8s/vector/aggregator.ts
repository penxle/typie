import * as k8s from '@pulumi/kubernetes';
import * as pulumi from '@pulumi/pulumi';
import { buckets } from '$aws/s3';
import { IAMServiceAccount } from '$components';
import { namespace } from './namespace';

const serviceAccount = new IAMServiceAccount('vector-aggregator', {
  metadata: {
    name: 'vector-aggregator',
    namespace: namespace.metadata.name,
  },
  spec: {
    policy: {
      Version: '2012-10-17',
      Statement: [
        {
          Effect: 'Allow',
          Action: ['s3:ListBucket'],
          Resource: [buckets.logs.arn],
        },
        {
          Effect: 'Allow',
          Action: ['s3:PutObject'],
          Resource: [pulumi.interpolate`${buckets.logs.arn}/*`],
        },
      ],
    },
  },
});

new k8s.helm.v4.Chart('vector-aggregator', {
  chart: 'vector',
  namespace: namespace.metadata.name,
  repositoryOpts: {
    repo: 'https://helm.vector.dev',
  },
  values: {
    role: 'Aggregator',
    replicas: 2,
    serviceAccount: {
      create: false,
      name: serviceAccount.metadata.name,
    },
    customConfig: {
      data_dir: '/vector-data-dir',
      sources: {
        vector: {
          type: 'vector',
          address: '0.0.0.0:6000',
        },
      },
      sinks: {
        // console: {
        //   inputs: ['*'],
        //   type: 'console',
        //   encoding: {
        //     codec: 'json',
        //   },
        // },
        aws_s3: {
          inputs: ['*'],
          type: 'aws_s3',
          bucket: buckets.logs.bucket,
          key_prefix: '%Y/%m/%d/',
          compression: 'zstd',
          encoding: {
            codec: 'json',
          },
        },
      },
    },
  },
});
