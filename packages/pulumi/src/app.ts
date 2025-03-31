import * as aws from '@pulumi/aws';
import * as k8s from '@pulumi/kubernetes';
import * as pulumi from '@pulumi/pulumi';
import { match } from 'ts-pattern';
import { DopplerSecret } from './doppler-secret';
import { IAMServiceAccount } from './iam-service-account';

type AppArgs = {
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
};

export class App extends pulumi.ComponentResource {
  public readonly service: k8s.core.v1.Service;

  constructor(name: string, args: AppArgs, opts?: pulumi.ComponentResourceOptions) {
    super('typie:index:App', name, {}, opts);

    const project = pulumi.getProject();
    const stack = pulumi.getStack();

    const namespace = match(stack)
      .with('prod', () => 'prod')
      .with('dev', () => 'dev')
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
            config: stack,
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
          },
        },
        spec: {
          ...(stack === 'dev' && { replicas: 1 }),
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

    if (stack === 'prod') {
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
