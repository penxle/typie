import * as aws from '@pulumi/aws';
import * as k8s from '@pulumi/kubernetes';
import * as pulumi from '@pulumi/pulumi';
import { match } from 'ts-pattern';

type AppArgs = {
  name: pulumi.Input<string>;

  image: {
    name: pulumi.Input<string>;
    version: pulumi.Input<string>;
  };

  resources: {
    cpu: pulumi.Input<string>;
    memory: pulumi.Input<string>;
  };

  env?: { name: pulumi.Input<string>; value: pulumi.Input<string> }[];
  secrets?: { token: pulumi.Input<string> };

  autoscale?: {
    minCount: pulumi.Input<number>;
    maxCount: pulumi.Input<number>;
    averageCpuUtilization: pulumi.Input<number>;
  };

  iam?: {
    policy: pulumi.Input<aws.iam.PolicyDocument>;
  };
};

export class App2 extends pulumi.ComponentResource {
  public readonly service: k8s.core.v1.Service;

  constructor(name: string, args: AppArgs, opts?: pulumi.ComponentResourceOptions) {
    super('typie:index:App2', name, {}, opts);

    const project = pulumi.getProject();
    const stack = pulumi.getStack();

    const namespace = match(stack)
      .with('prod', () => 'prod')
      .with('dev', () => 'dev')
      .run();

    let iamSecret;
    if (args.iam) {
      const user = new aws.iam.User(
        `${name}@k8s`,
        {
          name: pulumi.interpolate`${args.name}+${namespace}@k8s`,
        },
        { parent: this },
      );

      new aws.iam.UserPolicy(
        `${name}@k8s`,
        {
          user: user.name,
          policy: args.iam.policy,
        },
        { parent: this },
      );

      const accessKey = new aws.iam.AccessKey(
        `${name}@k8s`,
        {
          user: user.name,
        },
        { parent: this },
      );

      iamSecret = new k8s.core.v1.Secret(
        `${name}@iam`,
        {
          metadata: {
            name: pulumi.interpolate`${args.name}-iam`,
            namespace,
          },
          stringData: {
            AWS_REGION: 'ap-northeast-2',
            AWS_ACCESS_KEY_ID: accessKey.id,
            AWS_SECRET_ACCESS_KEY: accessKey.secret,
          },
        },
        { parent: this },
      );
    }

    let es;
    if (args.secrets) {
      const secret = new k8s.core.v1.Secret(
        name,
        {
          metadata: {
            name: pulumi.interpolate`${args.name}-doppler-token`,
            namespace,
          },
          stringData: {
            token: args.secrets.token,
          },
        },
        { parent: this },
      );

      const ss = new k8s.apiextensions.CustomResource(
        name,
        {
          apiVersion: 'external-secrets.io/v1',
          kind: 'SecretStore',

          metadata: {
            name: args.name,
            namespace,
          },

          spec: {
            provider: {
              doppler: {
                auth: {
                  secretRef: {
                    dopplerToken: {
                      name: secret.metadata.name,
                      key: 'token',
                    },
                  },
                },
              },
            },
          },
        },
        { parent: this },
      );

      es = new k8s.apiextensions.CustomResource(
        name,
        {
          apiVersion: 'external-secrets.io/v1',
          kind: 'ExternalSecret',
          metadata: {
            name: args.name,
            namespace,
          },
          spec: {
            refreshInterval: '1m',

            secretStoreRef: {
              kind: ss.kind,
              name: ss.metadata.name,
            },

            target: {
              name: args.name,
            },

            dataFrom: [{ find: { name: { regexp: '.*' } } }],
          },
        },
        { parent: this },
      );
    }

    const labels = { app: args.name };

    const service = new k8s.core.v1.Service(
      name,
      {
        metadata: {
          name: args.name,
          namespace,
        },
        spec: {
          type: 'ClusterIP',
          selector: labels,
          ports: [{ name: 'http', port: 80, targetPort: 3000 }],
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
            'reloader.stakater.com/auto': 'true',
          },
        },
        spec: {
          ...(stack === 'dev' && { replicas: 1 }),
          selector: { matchLabels: labels },
          revisionHistoryLimit: 2,
          template: {
            metadata: { labels },
            spec: {
              containers: [
                {
                  name: 'app',
                  image: pulumi.interpolate`${args.image.name}:${args.image.version}`,
                  env: [
                    { name: 'LISTEN_PORT', value: '3000' },
                    { name: 'PUBLIC_PULUMI_PROJECT', value: project },
                    { name: 'PUBLIC_PULUMI_STACK', value: stack },
                    { name: 'AWS_REGION', value: 'ap-northeast-2' },
                    ...(args.env ?? []),
                  ],
                  envFrom: [
                    ...(es ? [{ secretRef: { name: es.metadata.name } }] : []),
                    ...(iamSecret ? [{ secretRef: { name: iamSecret.metadata.name } }] : []),
                  ],
                  resources: {
                    requests: { cpu: args.resources.cpu },
                    limits: { memory: args.resources.memory },
                  },
                  livenessProbe: {
                    httpGet: { path: '/healthz/liveness', port: 3000 },
                    initialDelaySeconds: 10,
                    periodSeconds: 10,
                    successThreshold: 1,
                    failureThreshold: 6,
                  },
                  readinessProbe: {
                    httpGet: { path: '/healthz/readiness', port: 3000 },
                    initialDelaySeconds: 10,
                    periodSeconds: 10,
                    successThreshold: 1,
                    failureThreshold: 3,
                  },
                },
              ],
              topologySpreadConstraints: [
                {
                  maxSkew: 1,
                  topologyKey: 'kubernetes.io/hostname',
                  whenUnsatisfiable: 'ScheduleAnyway',
                  labelSelector: {
                    matchLabels: {
                      app: args.name,
                    },
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

    if (args.autoscale && stack === 'prod') {
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

    this.service = service;
  }
}
