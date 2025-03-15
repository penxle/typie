import * as glitter from '@glitter/pulumi';
import * as pulumi from '@pulumi/pulumi';

const config = new pulumi.Config('glitter');
const ref = new pulumi.StackReference('glitter/infrastructure/base');

new glitter.Service('api', {
  name: 'api',

  image: {
    name: '509399603331.dkr.ecr.ap-northeast-2.amazonaws.com/glitter',
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
          Action: ['ses:SendEmail'],
          Resource: [ref.getOutput('AWS_SES_EMAIL_IDENTITY'), ref.getOutput('AWS_SES_CONFIGURATION_SET')],
          Condition: {
            StringEquals: {
              'ses:FromAddress': 'hello@glitter.im',
              // 'ses:FromDisplayName': '글리터',
            },
          },
        },
      ],
    },
  },

  secret: {
    project: 'glitter-api',
  },

  ingress: {
    domain: {
      production: 'api.glitter.im',
      dev: 'api.glitter.pizza',
    },

    priority: {
      production: '11',
      dev: '111',
    },
  },
});
