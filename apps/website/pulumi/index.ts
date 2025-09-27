import * as aws from '@pulumi/aws';
import * as k8s from '@pulumi/kubernetes';
import * as pulumi from '@pulumi/pulumi';
import * as typie from '@typie/pulumi';

const stack = pulumi.getStack();
const config = new pulumi.Config();
const ref = new pulumi.StackReference('typie/infrastructure/base');

if (stack === 'prod') {
  const app = new typie.App('website', {
    name: 'website',

    image: {
      name: '509399603331.dkr.ecr.ap-northeast-2.amazonaws.com/website',
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
      rules: ['typie.co', 'auth.typie.co', 'typie.me', '*.typie.me'].map((host) => ({
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
} else if (stack === 'dev') {
  const provider = new k8s.Provider('bm', {
    kubeconfig: '~/.kube/config',
  });

  const app = new typie.App2(
    'website@bm',
    {
      name: 'website',

      image: {
        name: '509399603331.dkr.ecr.ap-northeast-2.amazonaws.com/website',
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
    },
    { provider },
  );

  new k8s.apiextensions.CustomResource(
    'website@bm',
    {
      apiVersion: 'gateway.networking.k8s.io/v1',
      kind: 'HTTPRoute',
      metadata: {
        name: 'website',
        namespace: app.service.metadata.namespace,
        annotations: {
          'external-dns.typie.io/enabled': 'true',
        },
      },
      spec: {
        parentRefs: [{ name: 'http', namespace: 'infra' }],
        hostnames: ['typie.dev', 'auth.typie.dev', 'typie.app', '*.typie.app'],
        rules: [
          {
            backendRefs: [
              {
                name: app.service.metadata.name,
                port: app.service.spec.ports[0].port,
              },
            ],
          },
        ],
      },
    },
    { provider },
  );

  new k8s.apiextensions.CustomResource(
    'www-redirect@bm',
    {
      apiVersion: 'gateway.networking.k8s.io/v1',
      kind: 'HTTPRoute',
      metadata: {
        name: 'www-redirect',
        namespace: app.service.metadata.namespace,
        annotations: {
          'external-dns.typie.io/enabled': 'true',
        },
      },
      spec: {
        parentRefs: [{ name: 'http', namespace: 'infra' }],
        hostnames: ['www.typie.dev', 'www.typie.app'],
        rules: [
          {
            matches: [
              {
                headers: [{ name: 'Host', value: 'www.typie.dev' }],
              },
            ],
            filters: [
              {
                type: 'RequestRedirect',
                requestRedirect: {
                  hostname: 'typie.dev',
                  statusCode: 301,
                },
              },
            ],
          },
          {
            matches: [
              {
                headers: [{ name: 'Host', value: 'www.typie.app' }],
              },
            ],
            filters: [
              {
                type: 'RequestRedirect',
                requestRedirect: {
                  hostname: 'typie.app',
                  statusCode: 301,
                },
              },
            ],
          },
        ],
      },
    },
    { provider },
  );
}
