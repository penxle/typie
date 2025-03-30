import * as pulumi from '@pulumi/pulumi';
import * as typie from '@typie/pulumi';

const config = new pulumi.Config('typie');
const ref = new pulumi.StackReference('typie/infrastructure/base');

new typie.Service('api', {
  name: 'api',

  image: {
    name: '509399603331.dkr.ecr.ap-northeast-2.amazonaws.com/typie',
    digest: config.require('digest'),
    command: ['bun', 'run', 'apps/api/index.js'],
  },

  resources: {
    cpu: '2',
    memory: '4Gi',
  },

  autoscale: {
    minCount: 2,
    maxCount: 20,
    averageCpuUtilization: 50,
  },

  iam: {
    base: {
      Version: '2012-10-17',
      Statement: [
        {
          Effect: 'Allow',
          Action: ['s3:GetObject', 's3:PutObject'],
          Resource: [pulumi.concat(ref.requireOutput('AWS_S3_BUCKET_UPLOADS_ARN'), '/*')],
        },
        {
          Effect: 'Allow',
          Action: ['s3:PutObject'],
          Resource: [pulumi.concat(ref.requireOutput('AWS_S3_BUCKET_USERCONTENTS_ARN'), '/*')],
        },
        {
          Effect: 'Allow',
          Action: ['ses:SendEmail'],
          Resource: [ref.getOutput('AWS_SES_EMAIL_IDENTITY'), ref.getOutput('AWS_SES_CONFIGURATION_SET')],
          Condition: {
            StringEquals: {
              'ses:FromAddress': 'hello@typie.co',
              'ses:FromDisplayName': 'Typie',
            },
          },
        },
      ],
    },
  },

  secret: {
    project: 'typie-api',
  },

  ingress: {
    domain: {
      production: ['api.typie.co'],
      dev: ['api.typie.dev'],
    },

    priority: {
      production: '11',
      dev: '111',
    },
  },
});
