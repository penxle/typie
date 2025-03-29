import * as aws from '@pulumi/aws';
import * as k8s from '@pulumi/kubernetes';
import * as pulumi from '@pulumi/pulumi';
import { match } from 'ts-pattern';
import { DopplerSecret } from './doppler-secret';
import { IAMServiceAccount } from './iam-service-account';

type ServiceArgs = {
  name: pulumi.Input<string>;

  image: {
    name: pulumi.Input<string>;
    digest: pulumi.Input<string>;
    command?: pulumi.Input<string[]>;
  };

  resources: {
    cpu: pulumi.Input<string>;
    memory: pulumi.Input<string>;
  };

  autoscale?: {
    minCount?: pulumi.Input<number>;
    maxCount?: pulumi.Input<number>;
    averageCpuUtilization?: pulumi.Input<number>;
  };

  iam?: {
    base?: pulumi.Input<aws.iam.PolicyDocument>;
    production?: pulumi.Input<aws.iam.PolicyDocument>;
    dev?: pulumi.Input<aws.iam.PolicyDocument>;
  };

  secret?: {
    project: pulumi.Input<string>;
  };

  ingress?: {
    path?: pulumi.Input<string>;

    domain: {
      production: pulumi.Input<string>;
      dev: pulumi.Input<string>;
    };

    priority: {
      production: pulumi.Input<string>;
      dev: pulumi.Input<string>;
    };

    cloudfront?: {
      production?: {
        domainZone: pulumi.Input<string>;
      };

      dev?: {
        domainZone: pulumi.Input<string>;
      };
    };
  };
};

export class Service extends pulumi.ComponentResource {
  constructor(name: string, args: ServiceArgs, opts?: pulumi.ComponentResourceOptions) {
    super('typie:index:Service', name, {}, opts);

    const project = pulumi.getProject();
    const stack = pulumi.getStack();
    const isProd = stack === 'prod';

    const namespace = match(stack)
      .with('prod', () => 'prod')
      .with('dev', () => 'dev')
      .run();

    const config = match(stack)
      .with('prod', () => 'prod')
      .with('dev', () => 'dev')
      .run();

    const cloudfront = match(stack)
      .with('prod', () => args.ingress?.cloudfront?.production)
      .with('dev', () => args.ingress?.cloudfront?.dev)
      .run();

    let secret;
    if (args.secret) {
      secret = new DopplerSecret(
        name,
        {
          metadata: {
            name: args.name,
            namespace,
          },
          spec: {
            project: args.secret.project,
            config,
          },
        },
        { parent: this },
      );
    }

    let serviceAccount;
    if (args.iam) {
      const policies = args.iam.base ? [args.iam.base] : [];
      const policy = match(stack)
        .with('prod', () => args.iam?.production)
        .with('dev', () => args.iam?.dev)
        .run();

      if (policy) {
        policies.push(policy);
      }

      if (policies.length > 0) {
        serviceAccount = new IAMServiceAccount(
          name,
          {
            metadata: {
              name: args.name,
              namespace,
            },
            spec: {
              policies,
            },
          },
          { parent: this },
        );
      }
    }

    const labels = { app: args.name };

    const service = new k8s.core.v1.Service(
      name,
      {
        metadata: {
          name: args.name,
          namespace,
          annotations: {
            'pulumi.com/skipAwait': 'true',
          },
        },
        spec: {
          type: 'NodePort',
          selector: labels,
          ports: [{ port: 3000 }],
        },
      },
      { parent: this },
    );

    const rollout = new k8s.apiextensions.CustomResource(
      name,
      {
        apiVersion: 'argoproj.io/v1alpha1',
        kind: 'Rollout',

        metadata: {
          name: args.name,
          namespace,
          annotations: {
            'pulumi.com/patchForce': 'true',
            'reloader.stakater.com/auto': 'true',
            ...(isProd && {
              'notifications.argoproj.io/subscribe.on-rollout-completed.slack': 'activities',
            }),
          },
        },
        spec: {
          ...(!isProd && { replicas: 1 }),
          selector: { matchLabels: labels },
          template: {
            metadata: { labels },
            spec: {
              ...(serviceAccount && { serviceAccountName: serviceAccount.metadata.name }),
              containers: [
                {
                  name: 'app',
                  image: pulumi.interpolate`${args.image.name}@${args.image.digest}`,
                  command: args.image.command,
                  env: [
                    { name: 'LISTEN_PORT', value: '3000' },
                    { name: 'PUBLIC_PULUMI_PROJECT', value: project },
                    { name: 'PUBLIC_PULUMI_STACK', value: stack },
                    { name: 'AWS_ROLE_SESSION_NAME', valueFrom: { fieldRef: { fieldPath: 'metadata.name' } } },
                  ],
                  ...(secret && { envFrom: [{ secretRef: { name: secret.metadata.name } }] }),
                  resources: {
                    requests: { cpu: args.resources.cpu },
                    limits: { memory: args.resources.memory },
                  },
                  readinessProbe: {
                    httpGet: { path: '/healthz', port: 3000 },
                  },
                },
              ],
            },
          },
          strategy: {
            blueGreen: {
              activeService: service.metadata.name,
            },
          },
        },
      },
      {
        parent: this,
      },
    );

    if (isProd) {
      new k8s.autoscaling.v2.HorizontalPodAutoscaler(
        name,
        {
          metadata: {
            name: args.name,
            namespace,
          },
          spec: {
            scaleTargetRef: {
              apiVersion: rollout.apiVersion,
              kind: rollout.kind,
              name: rollout.metadata.name,
            },
            minReplicas: args.autoscale?.minCount ?? 2,
            maxReplicas: args.autoscale?.maxCount ?? 10,
            metrics: [
              {
                type: 'Resource',
                resource: {
                  name: 'cpu',
                  target: {
                    type: 'Utilization',
                    averageUtilization: args.autoscale?.averageCpuUtilization ?? 50,
                  },
                },
              },
            ],
          },
        },
        { parent: this },
      );

      new k8s.policy.v1.PodDisruptionBudget(
        name,
        {
          metadata: {
            name: args.name,
            namespace,
          },
          spec: {
            selector: { matchLabels: labels },
            minAvailable: '50%',
          },
        },
        { parent: this },
      );
    }

    if (args.ingress) {
      /* eslint-disable @typescript-eslint/no-non-null-assertion */
      const domainName = match(stack)
        .with('prod', () => args.ingress!.domain.production)
        .with('dev', () => args.ingress!.domain.dev)
        .run();

      const priority = match(stack)
        .with('prod', () => args.ingress!.priority.production)
        .with('dev', () => args.ingress!.priority.dev)
        .run();
      /* eslint-enable @typescript-eslint/no-non-null-assertion */

      const ingress = new k8s.networking.v1.Ingress(
        name,
        {
          metadata: {
            name: args.name,
            namespace,
            annotations: {
              'alb.ingress.kubernetes.io/group.name': 'public-alb',
              'alb.ingress.kubernetes.io/group.order': priority,
              'alb.ingress.kubernetes.io/listen-ports': JSON.stringify([{ HTTPS: 443 }]),
              'alb.ingress.kubernetes.io/healthcheck-path': '/healthz',
              ...(cloudfront && { 'external-dns.alpha.kubernetes.io/ingress-hostname-source': 'annotation-only' }),
            },
          },
          spec: {
            ingressClassName: 'alb',
            rules: [
              {
                host: domainName,
                http: {
                  paths: [
                    {
                      path: args.ingress.path ?? '/',
                      pathType: 'Prefix',
                      backend: {
                        service: {
                          name: service.metadata.name,
                          port: { number: service.spec.ports[0].port },
                        },
                      },
                    },
                  ],
                },
              },
            ],
          },
        },
        { parent: this },
      );

      if (cloudfront) {
        const ref = new pulumi.StackReference('typie/infrastructure/base', {}, { parent: this });
        const provider = new aws.Provider('us-east-1', { region: 'us-east-1' }, { parent: this });

        const zone = aws.route53.getZoneOutput({ name: cloudfront.domainZone }, { parent: this });
        const certificate = aws.acm.getCertificateOutput(
          { domain: cloudfront.domainZone, statuses: ['ISSUED'] },
          { parent: this, provider },
        );

        const distribution = new aws.cloudfront.Distribution(
          name,
          {
            enabled: true,
            aliases: [domainName],
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
              acmCertificateArn: certificate.arn,
              sslSupportMethod: 'sni-only',
              minimumProtocolVersion: 'TLSv1.2_2021',
            },

            waitForDeployment: false,
          },
          { parent: this },
        );

        new aws.route53.Record(
          name,
          {
            name: domainName,
            type: 'A',
            zoneId: zone.zoneId,
            aliases: [
              {
                name: distribution.domainName,
                zoneId: distribution.hostedZoneId,
                evaluateTargetHealth: false,
              },
            ],
          },
          { parent: this },
        );
      }
    }
  }
}
