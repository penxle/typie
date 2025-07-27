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

  autoscale?: {
    minCount: pulumi.Input<number>;
    maxCount: pulumi.Input<number>;
    averageCpuUtilization: pulumi.Input<number>;
  };

  iam?: {
    policy: pulumi.Input<aws.iam.PolicyDocument>;
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

    let serviceAccount;
    if (args.iam) {
      const role = new aws.iam.Role(
        `${name}+${namespace}@eks`,
        {
          name: pulumi.interpolate`${args.name}+${namespace}@eks`,
          assumeRolePolicy: {
            Version: '2012-10-17',
            Statement: [
              {
                Effect: 'Allow',
                Principal: { Service: 'pods.eks.amazonaws.com' },
                Action: ['sts:AssumeRole', 'sts:TagSession'],
              },
            ],
          },
        },
        { parent: this },
      );

      new aws.iam.RolePolicy(
        `${name}+${namespace}@eks`,
        {
          role: role.name,
          policy: args.iam.policy,
        },
        { parent: this },
      );

      const assoc = new aws.eks.PodIdentityAssociation(
        `${name}+${namespace}@eks`,
        {
          clusterName: 'typie',
          namespace,
          roleArn: role.arn,
          serviceAccount: args.name,
        },
        { parent: this },
      );

      serviceAccount = new k8s.core.v1.ServiceAccount(
        name,
        {
          metadata: {
            name: assoc.serviceAccount,
            namespace,
          },
        },
        { parent: this },
      );
    }

    const es = new k8s.apiextensions.CustomResource(
      name,
      {
        apiVersion: 'external-secrets.io/v1',
        kind: 'ExternalSecret',
        metadata: {
          name: args.name,
          namespace,
        },
        spec: {
          refreshInterval: '5m',

          secretStoreRef: {
            kind: 'ClusterSecretStore',
            name: 'secrets-manager',
          },

          target: {
            name: args.name,
          },

          dataFrom: [
            {
              extract: {
                key: pulumi.interpolate`/apps/${project}/${stack}`,
              },
            },
          ],
        },
      },
      { parent: this },
    );

    const labels = { app: args.name };

    const service = new k8s.core.v1.Service(
      name,
      {
        metadata: {
          name: args.name,
          namespace,
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
                  image: pulumi.interpolate`${args.image.name}:${args.image.version}`,
                  env: [
                    { name: 'LISTEN_PORT', value: '3000' },
                    { name: 'PUBLIC_PULUMI_PROJECT', value: project },
                    { name: 'PUBLIC_PULUMI_STACK', value: stack },
                    { name: 'AWS_REGION', value: 'ap-northeast-2' },
                  ],
                  envFrom: [{ secretRef: { name: es.metadata.name } }],
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
