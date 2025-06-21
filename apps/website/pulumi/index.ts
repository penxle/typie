import * as aws from '@pulumi/aws';
import * as pulumi from '@pulumi/pulumi';
import * as typie from '@typie/pulumi';

const stack = pulumi.getStack();
const config = new pulumi.Config();
const ref = new pulumi.StackReference('typie/infrastructure/base');

new typie.Service('website', {
  name: 'website',

  image: {
    name: '509399603331.dkr.ecr.ap-northeast-2.amazonaws.com/website',
    version: config.require('version'),
  },

  resources: {
    cpu: '1024',
    memory: '2048',
  },

  autoscale: {
    minCount: 4,
    maxCount: 20,
    averageCpuUtilization: 80,
  },

  domains:
    stack === 'dev'
      ? ['typie.dev', 'auth.typie.dev', 'usersite.typie.dev', '*.usersite.typie.dev']
      : ['typie.co', 'auth.typie.co', 'typie.me', '*.typie.me'],

  env: {
    entries: [
      'PUBLIC_API_URL',
      'PUBLIC_AUTH_URL',
      'PUBLIC_MIXPANEL_TOKEN',
      'PUBLIC_OIDC_CLIENT_ID',
      'PUBLIC_USERSITE_HOST',
      'PUBLIC_WEBSITE_URL',
      'PUBLIC_WS_URL',
      'PRIVATE_OIDC_CLIENT_SECRET',
    ],
  },
});

if (stack === 'dev') {
  new aws.lb.ListenerRule('www.typie.dev', {
    listenerArn: ref.requireOutput('AWS_ELB_PUBLIC_LISTENER_ARN'),
    conditions: [{ hostHeader: { values: ['www.typie.dev'] } }],
    actions: [{ type: 'redirect', redirect: { host: 'typie.dev', statusCode: 'HTTP_301' } }],
  });

  new aws.route53.Record('typie.dev', {
    zoneId: ref.requireOutput('AWS_ROUTE53_TYPIE_DEV_ZONE_ID'),
    name: 'typie.dev',
    type: 'A',
    aliases: [
      {
        name: ref.requireOutput('AWS_ELB_PUBLIC_DNS_NAME'),
        zoneId: ref.requireOutput('AWS_ELB_PUBLIC_ZONE_ID'),
        evaluateTargetHealth: true,
      },
    ],
  });

  new aws.route53.Record('www.typie.dev', {
    zoneId: ref.requireOutput('AWS_ROUTE53_TYPIE_DEV_ZONE_ID'),
    name: 'www.typie.dev',
    type: 'A',
    aliases: [
      {
        name: ref.requireOutput('AWS_ELB_PUBLIC_DNS_NAME'),
        zoneId: ref.requireOutput('AWS_ELB_PUBLIC_ZONE_ID'),
        evaluateTargetHealth: true,
      },
    ],
  });

  new aws.route53.Record('auth.typie.dev', {
    zoneId: ref.requireOutput('AWS_ROUTE53_TYPIE_DEV_ZONE_ID'),
    name: 'auth.typie.dev',
    type: 'A',
    aliases: [
      {
        name: ref.requireOutput('AWS_ELB_PUBLIC_DNS_NAME'),
        zoneId: ref.requireOutput('AWS_ELB_PUBLIC_ZONE_ID'),
        evaluateTargetHealth: true,
      },
    ],
  });

  new aws.route53.Record('usersite.typie.dev', {
    zoneId: ref.requireOutput('AWS_ROUTE53_TYPIE_DEV_ZONE_ID'),
    name: 'usersite.typie.dev',
    type: 'A',
    aliases: [
      {
        name: ref.requireOutput('AWS_ELB_PUBLIC_DNS_NAME'),
        zoneId: ref.requireOutput('AWS_ELB_PUBLIC_ZONE_ID'),
        evaluateTargetHealth: true,
      },
    ],
  });

  new aws.route53.Record('*.usersite.typie.dev', {
    zoneId: ref.requireOutput('AWS_ROUTE53_TYPIE_DEV_ZONE_ID'),
    name: '*.usersite.typie.dev',
    type: 'A',
    aliases: [
      {
        name: ref.requireOutput('AWS_ELB_PUBLIC_DNS_NAME'),
        zoneId: ref.requireOutput('AWS_ELB_PUBLIC_ZONE_ID'),
        evaluateTargetHealth: true,
      },
    ],
  });
} else {
  const typie_co = new aws.cloudfront.Distribution('typie.co', {
    enabled: true,
    aliases: ['typie.co', 'auth.typie.co'],
    httpVersion: 'http2and3',

    origins: [
      {
        originId: 'alb',
        domainName: ref.requireOutput('AWS_ELB_PUBLIC_DNS_NAME'),
        customOriginConfig: {
          httpPort: 80,
          httpsPort: 443,
          originProtocolPolicy: 'https-only',
          originSslProtocols: ['TLSv1.2'],
          originReadTimeout: 60,
          originKeepaliveTimeout: 60,
        },
      },
    ],

    defaultCacheBehavior: {
      targetOriginId: 'alb',
      compress: false,
      viewerProtocolPolicy: 'redirect-to-https',
      allowedMethods: ['GET', 'HEAD', 'OPTIONS', 'PUT', 'POST', 'PATCH', 'DELETE'],
      cachedMethods: ['GET', 'HEAD', 'OPTIONS'],
      cachePolicyId: ref.requireOutput('AWS_CLOUDFRONT_DYNAMIC_CACHE_POLICY_ID'),
      originRequestPolicyId: ref.requireOutput('AWS_CLOUDFRONT_DYNAMIC_ORIGIN_REQUEST_POLICY_ID'),
      responseHeadersPolicyId: ref.requireOutput('AWS_CLOUDFRONT_DYNAMIC_RESPONSE_HEADERS_POLICY_ID'),
    },

    restrictions: {
      geoRestriction: {
        restrictionType: 'none',
      },
    },

    viewerCertificate: {
      acmCertificateArn: ref.requireOutput('AWS_CLOUDFRONT_TYPIE_CO_CERTIFICATE_ARN'),
      sslSupportMethod: 'sni-only',
      minimumProtocolVersion: 'TLSv1.2_2021',
    },

    waitForDeployment: false,
  });

  const typie_me = new aws.cloudfront.Distribution('typie.me', {
    enabled: true,
    aliases: ['typie.me', '*.typie.me'],
    httpVersion: 'http2and3',

    origins: [
      {
        originId: 'alb',
        domainName: ref.requireOutput('AWS_ELB_PUBLIC_DNS_NAME'),
        customOriginConfig: {
          httpPort: 80,
          httpsPort: 443,
          originProtocolPolicy: 'https-only',
          originSslProtocols: ['TLSv1.2'],
          originReadTimeout: 60,
          originKeepaliveTimeout: 60,
        },
      },
    ],

    defaultCacheBehavior: {
      targetOriginId: 'alb',
      compress: false,
      viewerProtocolPolicy: 'redirect-to-https',
      allowedMethods: ['GET', 'HEAD', 'OPTIONS', 'PUT', 'POST', 'PATCH', 'DELETE'],
      cachedMethods: ['GET', 'HEAD', 'OPTIONS'],
      cachePolicyId: ref.requireOutput('AWS_CLOUDFRONT_DYNAMIC_CACHE_POLICY_ID'),
      originRequestPolicyId: ref.requireOutput('AWS_CLOUDFRONT_DYNAMIC_ORIGIN_REQUEST_POLICY_ID'),
      responseHeadersPolicyId: ref.requireOutput('AWS_CLOUDFRONT_DYNAMIC_RESPONSE_HEADERS_POLICY_ID'),
    },

    restrictions: {
      geoRestriction: {
        restrictionType: 'none',
      },
    },

    viewerCertificate: {
      acmCertificateArn: ref.requireOutput('AWS_CLOUDFRONT_TYPIE_ME_CERTIFICATE_ARN'),
      sslSupportMethod: 'sni-only',
      minimumProtocolVersion: 'TLSv1.2_2021',
    },

    waitForDeployment: false,
  });

  new aws.lb.ListenerRule('www.typie.co', {
    listenerArn: ref.requireOutput('AWS_ELB_PUBLIC_LISTENER_ARN'),
    conditions: [{ hostHeader: { values: ['www.typie.co'] } }],
    actions: [{ type: 'redirect', redirect: { host: 'typie.co', statusCode: 'HTTP_301' } }],
  });

  new aws.lb.ListenerRule('www.typie.me', {
    listenerArn: ref.requireOutput('AWS_ELB_PUBLIC_LISTENER_ARN'),
    conditions: [{ hostHeader: { values: ['www.typie.me'] } }],
    actions: [{ type: 'redirect', redirect: { host: 'typie.me', statusCode: 'HTTP_301' } }],
  });

  new aws.route53.Record('typie.co', {
    name: 'typie.co',
    type: 'A',
    zoneId: ref.requireOutput('AWS_ROUTE53_TYPIE_CO_ZONE_ID'),
    aliases: [
      {
        name: typie_co.domainName,
        zoneId: typie_co.hostedZoneId,
        evaluateTargetHealth: false,
      },
    ],
  });

  new aws.route53.Record('www.typie.co', {
    name: 'www.typie.co',
    type: 'A',
    zoneId: ref.requireOutput('AWS_ROUTE53_TYPIE_CO_ZONE_ID'),
    aliases: [
      {
        name: ref.requireOutput('AWS_ELB_PUBLIC_DNS_NAME'),
        zoneId: ref.requireOutput('AWS_ELB_PUBLIC_ZONE_ID'),
        evaluateTargetHealth: false,
      },
    ],
  });

  new aws.route53.Record('auth.typie.co', {
    name: 'auth.typie.co',
    type: 'A',
    zoneId: ref.requireOutput('AWS_ROUTE53_TYPIE_CO_ZONE_ID'),
    aliases: [
      {
        name: typie_co.domainName,
        zoneId: typie_co.hostedZoneId,
        evaluateTargetHealth: false,
      },
    ],
  });

  new aws.route53.Record('typie.me', {
    name: 'typie.me',
    type: 'A',
    zoneId: ref.requireOutput('AWS_ROUTE53_TYPIE_ME_ZONE_ID'),
    aliases: [
      {
        name: typie_me.domainName,
        zoneId: typie_me.hostedZoneId,
        evaluateTargetHealth: false,
      },
    ],
  });

  new aws.route53.Record('www.typie.me', {
    name: 'www.typie.me',
    type: 'A',
    zoneId: ref.requireOutput('AWS_ROUTE53_TYPIE_ME_ZONE_ID'),
    aliases: [
      {
        name: ref.requireOutput('AWS_ELB_PUBLIC_DNS_NAME'),
        zoneId: ref.requireOutput('AWS_ELB_PUBLIC_ZONE_ID'),
        evaluateTargetHealth: false,
      },
    ],
  });

  new aws.route53.Record('*.typie.me', {
    name: '*.typie.me',
    type: 'A',
    zoneId: ref.requireOutput('AWS_ROUTE53_TYPIE_ME_ZONE_ID'),
    aliases: [
      {
        name: typie_me.domainName,
        zoneId: typie_me.hostedZoneId,
        evaluateTargetHealth: false,
      },
    ],
  });
}
