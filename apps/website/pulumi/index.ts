import * as aws from '@pulumi/aws';
import * as k8s from '@pulumi/kubernetes';
import * as pulumi from '@pulumi/pulumi';
import * as typie from '@typie/pulumi';

const stack = pulumi.getStack();
const config = new pulumi.Config();
const ref = new pulumi.StackReference('typie/infrastructure/base');

const app = new typie.App('website', {
  name: 'website',

  image: {
    name: '509399603331.dkr.ecr.ap-northeast-2.amazonaws.com/website',
    version: config.require('version'),
  },

  resources: {
    cpu: '500m',
    memory: '1Gi',
  },

  autoscale: {
    minCount: 4,
    maxCount: 20,
    averageCpuUtilization: 50,
  },
});

const ingress = new k8s.networking.v1.Ingress('website', {
  metadata: {
    name: 'website',
    namespace: app.service.metadata.namespace,
    annotations: {
      'alb.ingress.kubernetes.io/group.name': 'public-alb',
      'alb.ingress.kubernetes.io/group.order': stack === 'prod' ? '20' : '120',
      'alb.ingress.kubernetes.io/listen-ports': JSON.stringify([{ HTTPS: 443 }]),
      'alb.ingress.kubernetes.io/healthcheck-path': '/healthz',
      ...(stack === 'prod' && { 'external-dns.alpha.kubernetes.io/ingress-hostname-source': 'annotation-only' }),
    },
  },
  spec: {
    ingressClassName: 'alb',
    rules: (stack === 'prod'
      ? ['typie.co', 'auth.typie.co', 'typie.me', '*.typie.me']
      : ['typie.dev', 'auth.typie.dev', 'usersite.typie.dev', '*.usersite.typie.dev']
    ).map((host) => ({
      host,
      http: {
        paths: [
          {
            path: '/',
            pathType: 'Prefix',
            backend: {
              service: {
                name: app.service.metadata.name,
                port: { number: app.service.spec.ports[0].port },
              },
            },
          },
        ],
      },
    })),
  },
});

new k8s.networking.v1.Ingress('www.website', {
  metadata: {
    name: 'www.website',
    namespace: app.service.metadata.namespace,
    annotations: {
      'alb.ingress.kubernetes.io/group.name': 'public-alb',
      'alb.ingress.kubernetes.io/group.order': stack === 'prod' ? '21' : '121',
      'alb.ingress.kubernetes.io/listen-ports': JSON.stringify([{ HTTPS: 443 }]),
      'alb.ingress.kubernetes.io/actions.redirect': pulumi.jsonStringify({
        type: 'redirect',
        redirectConfig: {
          host: stack === 'prod' ? 'typie.co' : 'typie.dev',
          path: '/',
          statusCode: 'HTTP_301',
        },
      }),
    },
  },
  spec: {
    ingressClassName: 'alb',
    rules: (stack === 'prod' ? ['www.typie.co', 'www.typie.me'] : ['www.typie.dev']).map((host) => ({
      host,
      http: {
        paths: [
          {
            path: '/',
            pathType: 'Prefix',
            backend: {
              service: {
                name: 'redirect',
                port: { name: 'use-annotation' },
              },
            },
          },
        ],
      },
    })),
  },
});

if (stack === 'prod') {
  const typie_co = new aws.cloudfront.Distribution('typie.co', {
    enabled: true,
    aliases: ['typie.co', 'auth.typie.co'],
    httpVersion: 'http2and3',

    origins: [
      {
        originId: 'alb',
        domainName: ingress.status.loadBalancer.ingress[0].hostname,
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
        domainName: ingress.status.loadBalancer.ingress[0].hostname,
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

  new aws.route53.Record('typie.co', {
    name: 'typie.co',
    type: 'A',
    zoneId: ref.requireOutput('AWS_ROUTE53_TYPIE_CO_ZONE_ID'),
    aliases: [
      {
        name: typie_co.domainName,
        zoneId: typie_co.hostedZoneId,
        evaluateTargetHealth: true,
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
        evaluateTargetHealth: true,
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
        evaluateTargetHealth: true,
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
        evaluateTargetHealth: true,
      },
    ],
  });
}
