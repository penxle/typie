import * as k8s from '@pulumi/kubernetes';
import * as pulumi from '@pulumi/pulumi';
import dedent from 'dedent';

type SentinelArgs = {
  name: pulumi.Input<string>;
  namespace: pulumi.Input<string>;

  replicas: pulumi.Input<number>;
  hostname: pulumi.Input<string>;
};

class Sentinel extends pulumi.ComponentResource {
  constructor(name: string, args: SentinelArgs, opts?: pulumi.ComponentResourceOptions) {
    super('typie:valkey:Sentinel', name, args, opts);

    new k8s.helm.v4.Chart(
      name,
      {
        name: args.name,

        chart: 'valkey',
        namespace: args.namespace,
        repositoryOpts: {
          repo: 'https://charts.bitnami.com/bitnami',
        },

        values: {
          architecture: 'replication',

          commonConfiguration: dedent`
            maxmemory 2gb
            maxmemory-policy noeviction
          `,

          auth: {
            enabled: false,
          },

          sentinel: {
            enabled: true,
            primarySet: 'primary',
          },

          primary: {
            replicaCount: 1,

            resources: {
              requests: { cpu: '1' },
              limits: { memory: '2Gi' },
            },

            persistence: {
              enabled: false,
            },
          },

          replica: {
            replicaCount: args.replicas,

            resources: {
              requests: { cpu: '1' },
              limits: { memory: '2Gi' },
            },

            persistence: {
              enabled: false,
            },
          },

          metrics: {
            enabled: true,
            serviceMonitor: { enabled: true },
            podMonitor: { enabled: true },
            prometheusRule: { enabled: true },
          },
        },
      },
      { parent: this },
    );
  }
}

new Sentinel('redis@dev', {
  name: 'valkey',
  namespace: 'dev',

  replicas: 1,
  hostname: 'dev.redis.typie.io',
});

new Sentinel('redis@prod', {
  name: 'valkey',
  namespace: 'prod',

  replicas: 3,
  hostname: 'redis.typie.io',
});
