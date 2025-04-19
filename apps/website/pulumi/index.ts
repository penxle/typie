import * as aws from '@pulumi/aws';
import * as k8s from '@pulumi/kubernetes';
import * as pulumi from '@pulumi/pulumi';
import * as typie from '@typie/pulumi';
import { match } from 'ts-pattern';

const stack = pulumi.getStack();
const config = new pulumi.Config('typie');
const ref = new pulumi.StackReference('typie/infrastructure/base');

const app = new typie.App('website', {
  name: 'website',

  image: {
    name: '509399603331.dkr.ecr.ap-northeast-2.amazonaws.com/typie',
    digest: config.require('digest'),
    command: ['bun', 'run', 'apps/website/index.js'],
  },

  resources: {
    cpu: '500m',
    memory: '1Gi',
  },

  autoscale: {
    minCount: 2,
    maxCount: 20,
    averageCpuUtilization: 50,
  },

  secret: {
    project: 'typie-website',
  },
});

const provider = new aws.Provider('us-east-1', { region: 'us-east-1' });

const websiteIngress = new k8s.networking.v1.Ingress('website', {
  metadata: {
    name: 'website',
    namespace: app.service.metadata.namespace,
    annotations: {
      'alb.ingress.kubernetes.io/group.name': 'public-alb',
      'alb.ingress.kubernetes.io/group.order': '20',
      'alb.ingress.kubernetes.io/listen-ports': JSON.stringify([{ HTTPS: 443 }]),
      'alb.ingress.kubernetes.io/healthcheck-path': '/healthz',
      ...(stack === 'prod' && { 'external-dns.alpha.kubernetes.io/ingress-hostname-source': 'annotation-only' }),
    },
  },
  spec: {
    ingressClassName: 'alb',
    rules: [
      {
        host: match(stack)
          .with('prod', () => 'typie.co')
          .with('dev', () => 'typie.dev')
          .run(),
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
      },
    ],
  },
});

const usersiteIngress = new k8s.networking.v1.Ingress('usersite', {
  metadata: {
    name: 'usersite',
    namespace: app.service.metadata.namespace,
    annotations: {
      'alb.ingress.kubernetes.io/group.name': 'public-alb',
      'alb.ingress.kubernetes.io/group.order': '30',
      'alb.ingress.kubernetes.io/listen-ports': JSON.stringify([{ HTTPS: 443 }]),
      'alb.ingress.kubernetes.io/healthcheck-path': '/healthz',
      ...(stack === 'prod' && { 'external-dns.alpha.kubernetes.io/ingress-hostname-source': 'annotation-only' }),
    },
  },
  spec: {
    ingressClassName: 'alb',
    rules: [
      {
        host: match(stack)
          .with('prod', () => 'typie.me')
          .with('dev', () => 'usersite.typie.dev')
          .run(),
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
      },
      {
        host: match(stack)
          .with('prod', () => '*.typie.me')
          .with('dev', () => '*.usersite.typie.dev')
          .run(),
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
      },
    ],
  },
});

const authIngress = new k8s.networking.v1.Ingress('auth', {
  metadata: {
    name: 'auth',
    namespace: app.service.metadata.namespace,
    annotations: {
      'alb.ingress.kubernetes.io/group.name': 'public-alb',
      'alb.ingress.kubernetes.io/group.order': '40',
      'alb.ingress.kubernetes.io/listen-ports': JSON.stringify([{ HTTPS: 443 }]),
      'alb.ingress.kubernetes.io/healthcheck-path': '/healthz',
      ...(stack === 'prod' && { 'external-dns.alpha.kubernetes.io/ingress-hostname-source': 'annotation-only' }),
    },
  },
  spec: {
    ingressClassName: 'alb',
    rules: [
      {
        host: match(stack)
          .with('prod', () => 'auth.typie.co')
          .with('dev', () => 'auth.typie.dev')
          .run(),
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
      },
    ],
  },
});

if (stack === 'prod') {
  const websiteZone = aws.route53.getZoneOutput({ name: 'typie.co' });
  const websiteCertificate = aws.acm.getCertificateOutput({ domain: 'typie.co', statuses: ['ISSUED'] }, { provider });

  const usersiteZone = aws.route53.getZoneOutput({ name: 'typie.me' });
  const usersiteCertificate = aws.acm.getCertificateOutput({ domain: 'typie.me', statuses: ['ISSUED'] }, { provider });

  const authZone = aws.route53.getZoneOutput({ name: 'typie.co' });
  const authCertificate = aws.acm.getCertificateOutput({ domain: 'typie.co', statuses: ['ISSUED'] }, { provider });

  const websiteDistribution = new aws.cloudfront.Distribution('website', {
    enabled: true,
    aliases: ['typie.co'],
    httpVersion: 'http2and3',

    origins: [
      {
        originId: 'alb',
        domainName: websiteIngress.status.loadBalancer.ingress[0].hostname,
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
      compress: true,
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
      acmCertificateArn: websiteCertificate.arn,
      sslSupportMethod: 'sni-only',
      minimumProtocolVersion: 'TLSv1.2_2021',
    },

    waitForDeployment: false,
  });

  const usersiteDistribution = new aws.cloudfront.Distribution('usersite', {
    enabled: true,
    aliases: ['typie.me', '*.typie.me'],
    httpVersion: 'http2and3',

    origins: [
      {
        originId: 'alb',
        domainName: usersiteIngress.status.loadBalancer.ingress[0].hostname,
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
      compress: true,
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
      acmCertificateArn: usersiteCertificate.arn,
      sslSupportMethod: 'sni-only',
      minimumProtocolVersion: 'TLSv1.2_2021',
    },

    waitForDeployment: false,
  });

  const authDistribution = new aws.cloudfront.Distribution('auth', {
    enabled: true,
    aliases: ['auth.typie.co'],
    httpVersion: 'http2and3',

    origins: [
      {
        originId: 'alb',
        domainName: authIngress.status.loadBalancer.ingress[0].hostname,
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
      compress: true,
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
      acmCertificateArn: authCertificate.arn,
      sslSupportMethod: 'sni-only',
      minimumProtocolVersion: 'TLSv1.2_2021',
    },

    waitForDeployment: false,
  });

  new aws.route53.Record('website', {
    name: 'typie.co',
    type: 'A',
    zoneId: websiteZone.zoneId,
    aliases: [
      {
        name: websiteDistribution.domainName,
        zoneId: websiteDistribution.hostedZoneId,
        evaluateTargetHealth: false,
      },
    ],
  });

  new aws.route53.Record('usersite-apex', {
    name: 'typie.me',
    type: 'A',
    zoneId: usersiteZone.zoneId,
    aliases: [
      {
        name: usersiteDistribution.domainName,
        zoneId: usersiteDistribution.hostedZoneId,
        evaluateTargetHealth: false,
      },
    ],
  });

  new aws.route53.Record('usersite-wildcard', {
    name: '*.typie.me',
    type: 'A',
    zoneId: usersiteZone.zoneId,
    aliases: [
      {
        name: usersiteDistribution.domainName,
        zoneId: usersiteDistribution.hostedZoneId,
        evaluateTargetHealth: false,
      },
    ],
  });

  new aws.route53.Record('auth', {
    name: 'auth.typie.co',
    type: 'A',
    zoneId: authZone.zoneId,
    aliases: [
      {
        name: authDistribution.domainName,
        zoneId: authDistribution.hostedZoneId,
        evaluateTargetHealth: false,
      },
    ],
  });
}

new typie.Redirect('www.website', {
  name: 'www.website',

  priority: {
    production: '21',
    dev: '21',
  },

  production: {
    from: { host: 'www.typie.co' },
    to: { host: 'typie.co' },
  },

  dev: {
    from: { host: 'www.typie.dev' },
    to: { host: 'typie.dev' },
  },

  code: 301,
});

new typie.Redirect('www.usersite', {
  name: 'www.usersite',

  priority: {
    production: '29',
    dev: '29',
  },

  production: {
    from: { host: 'www.typie.me' },
    to: { host: 'typie.me' },
  },

  code: 301,
});
