import * as k8s from '@pulumi/kubernetes';
import * as pulumi from '@pulumi/pulumi';
import * as typie from '@typie/pulumi';

const stack = pulumi.getStack();
const config = new pulumi.Config();
const ref = new pulumi.StackReference('typie/infrastructure/base');

const app = new typie.App('api', {
  name: 'api',

  image: {
    name: '509399603331.dkr.ecr.ap-northeast-2.amazonaws.com/api',
    version: config.require('version'),
  },

  resources: {
    cpu: '1',
    memory: '2Gi',
  },

  env: [{ name: 'NO_WORKER', value: 'true' }],
  secrets: {
    token: config.requireSecret('doppler-token'),
  },

  autoscale: {
    minCount: 4,
    maxCount: 20,
    averageCpuUtilization: 80,
  },

  iam: {
    policy: {
      Version: '2012-10-17',
      Statement: [
        {
          Effect: 'Allow',
          Action: ['s3:GetObject', 's3:PutObject'],
          Resource: [pulumi.concat(ref.requireOutput('AWS_S3_BUCKET_UPLOADS_ARN'), '/*')],
        },
        {
          Effect: 'Allow',
          Action: ['s3:GetObject', 's3:PutObject', 's3:GetObjectTagging', 's3:PutObjectTagging'],
          Resource: [pulumi.concat(ref.requireOutput('AWS_S3_BUCKET_USERCONTENTS_ARN'), '/*')],
        },
        {
          Effect: 'Allow',
          Action: ['s3:GetObject', 's3:PutObject', 's3:GetObjectTagging', 's3:PutObjectTagging'],
          Resource: [pulumi.concat(ref.requireOutput('AWS_S3_BUCKET_MISC_ARN'), '/*')],
        },
        {
          Effect: 'Allow',
          Action: ['ses:SendEmail'],
          Resource: [ref.getOutput('AWS_SES_EMAIL_IDENTITY'), ref.getOutput('AWS_SES_CONFIGURATION_SET')],
          Condition: {
            StringEquals: {
              'ses:FromAddress': 'hello@typie.co',
            },
          },
        },
        {
          Effect: 'Allow',
          Action: ['ce:GetCostAndUsage'],
          Resource: '*',
        },
      ],
    },
  },
});

new typie.App('worker', {
  name: 'worker',

  image: {
    name: '509399603331.dkr.ecr.ap-northeast-2.amazonaws.com/api',
    version: config.require('version'),
  },

  resources: {
    cpu: '1',
    memory: '2Gi',
  },

  secrets: {
    token: config.requireSecret('doppler-token'),
  },

  autoscale: {
    minCount: 4,
    maxCount: 20,
    averageCpuUtilization: 80,
  },

  iam: {
    policy: {
      Version: '2012-10-17',
      Statement: [
        {
          Effect: 'Allow',
          Action: ['s3:GetObject', 's3:PutObject'],
          Resource: [pulumi.concat(ref.requireOutput('AWS_S3_BUCKET_UPLOADS_ARN'), '/*')],
        },
        {
          Effect: 'Allow',
          Action: ['s3:GetObject', 's3:PutObject', 's3:GetObjectTagging', 's3:PutObjectTagging'],
          Resource: [pulumi.concat(ref.requireOutput('AWS_S3_BUCKET_USERCONTENTS_ARN'), '/*')],
        },
        {
          Effect: 'Allow',
          Action: ['s3:GetObject', 's3:PutObject'],
          Resource: [pulumi.concat(ref.requireOutput('AWS_S3_BUCKET_MISC_ARN'), '/*')],
        },
        {
          Effect: 'Allow',
          Action: ['ses:SendEmail'],
          Resource: [ref.getOutput('AWS_SES_EMAIL_IDENTITY'), ref.getOutput('AWS_SES_CONFIGURATION_SET')],
          Condition: {
            StringEquals: {
              'ses:FromAddress': 'hello@typie.co',
            },
          },
        },
        {
          Effect: 'Allow',
          Action: ['ce:GetCostAndUsage'],
          Resource: '*',
        },
      ],
    },
  },
});

new k8s.apiextensions.CustomResource('api', {
  apiVersion: 'monitoring.coreos.com/v1',
  kind: 'ServiceMonitor',
  metadata: {
    name: 'api',
    namespace: app.service.metadata.namespace,
  },
  spec: {
    selector: {
      matchLabels: app.service.metadata.labels,
    },
    endpoints: [
      { port: 'http', path: '/metrics/bullmq' },
      { port: 'http', path: '/graphql/metrics' },
    ],
  },
});

new k8s.networking.v1.Ingress('api', {
  metadata: {
    name: 'api',
    namespace: app.service.metadata.namespace,
    annotations: {
      'alb.ingress.kubernetes.io/group.name': 'public-alb',
      'alb.ingress.kubernetes.io/group.order': stack === 'prod' ? '10' : '110',
      'alb.ingress.kubernetes.io/listen-ports': JSON.stringify([{ HTTPS: 443 }]),
      'alb.ingress.kubernetes.io/healthcheck-path': '/healthz',
    },
  },
  spec: {
    ingressClassName: 'alb',
    rules: [
      {
        host: stack === 'prod' ? 'api.typie.co' : 'api.typie.dev',
        http: {
          paths: [
            {
              path: '/',
              pathType: 'Prefix',
              backend: {
                service: {
                  name: app.service.metadata.name,
                  port: { name: 'http' },
                },
              },
            },
          ],
        },
      },
    ],
  },
});
