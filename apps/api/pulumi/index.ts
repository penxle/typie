import * as aws from '@pulumi/aws';
import * as pulumi from '@pulumi/pulumi';
import * as typie from '@typie/pulumi';

const stack = pulumi.getStack();
const config = new pulumi.Config();
const ref = new pulumi.StackReference('typie/infrastructure/base');

new typie.Service('api', {
  name: 'api',

  image: {
    name: '509399603331.dkr.ecr.ap-northeast-2.amazonaws.com/api',
    version: config.require('version'),
  },

  resources: {
    cpu: '1024',
    memory: '2048',
  },

  autoscale: {
    minCount: 2,
    maxCount: 20,
    averageCpuUtilization: 80,
  },

  domains: stack === 'dev' ? ['api.typie.dev'] : ['api.typie.co'],

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
          Action: ['s3:PutObject'],
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
      ],
    },
  },

  env: {
    entries: [
      'APPLE_APP_APPLE_ID',
      'APPLE_APP_BUNDLE_ID',
      'APPLE_IAP_ISSUER_ID',
      'APPLE_IAP_KEY_ID',
      'APPLE_IAP_PRIVATE_KEY',
      'APPLE_SIGN_IN_KEY_ID',
      'APPLE_SIGN_IN_PRIVATE_KEY',
      'APPLE_TEAM_ID',
      'AUTH_URL',
      'DATABASE_URL',
      'GOOGLE_OAUTH_CLIENT_ID',
      'GOOGLE_OAUTH_CLIENT_SECRET',
      'GOOGLE_PLAY_PACKAGE_NAME',
      'GOOGLE_SERVICE_ACCOUNT',
      'IFRAMELY_API_KEY',
      'KAKAO_CLIENT_ID',
      'KAKAO_CLIENT_SECRET',
      'MEILISEARCH_API_KEY',
      'MEILISEARCH_URL',
      'NAVER_CLIENT_ID',
      'NAVER_CLIENT_SECRET',
      'OIDC_CLIENT_ID',
      'OIDC_CLIENT_SECRET',
      'OIDC_JWK',
      'PORTONE_API_SECRET',
      'PORTONE_CHANNEL_KEY',
      'REDIS_URL',
      'SENTRY_DSN',
      'SLACK_WEBHOOK_URL',
      'USERSITE_URL',
      'WEBSITE_URL',
    ],
  },
});

if (stack === 'dev') {
  new aws.route53.Record('api.typie.dev', {
    zoneId: ref.requireOutput('AWS_ROUTE53_TYPIE_DEV_ZONE_ID'),
    type: 'A',
    name: 'api.typie.dev',
    aliases: [
      {
        name: ref.requireOutput('AWS_ELB_PUBLIC_DNS_NAME'),
        zoneId: ref.requireOutput('AWS_ELB_PUBLIC_ZONE_ID'),
        evaluateTargetHealth: true,
      },
    ],
  });
} else {
  new aws.route53.Record('api.typie.co', {
    zoneId: ref.requireOutput('AWS_ROUTE53_TYPIE_CO_ZONE_ID'),
    type: 'A',
    name: 'api.typie.co',
    aliases: [
      {
        name: ref.requireOutput('AWS_ELB_PUBLIC_DNS_NAME'),
        zoneId: ref.requireOutput('AWS_ELB_PUBLIC_ZONE_ID'),
        evaluateTargetHealth: true,
      },
    ],
  });
}
