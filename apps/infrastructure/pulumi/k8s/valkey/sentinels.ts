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
            maxmemory 3gb
            maxmemory-policy noeviction

            appendonly yes
            appendfsync everysec

            save ""
          `,

          useHostnames: false,

          auth: {
            enabled: false,
          },

          sentinel: {
            enabled: true,
            primarySet: 'primary',

            resources: {
              requests: { cpu: '500m' },
              limits: { memory: '1Gi' },
            },

            service: {
              type: 'LoadBalancer',
              loadBalancerClass: 'tailscale',

              annotations: {
                'external-dns.alpha.kubernetes.io/hostname': args.hostname,
                'tailscale.com/proxy-group': 'ingress',
              },
            },
          },

          primary: {
            replicaCount: 1,

            resources: {
              requests: { cpu: '2' },
              limits: { memory: '4Gi' },
            },

            persistence: {
              storageClass: 'gp3',
              size: '20Gi',
            },

            affinity: {
              podAntiAffinity: {
                requiredDuringSchedulingIgnoredDuringExecution: [
                  {
                    labelSelector: {
                      matchExpressions: [{ key: 'app.kubernetes.io/name', operator: 'In', values: [args.name] }],
                    },
                    topologyKey: 'kubernetes.io/hostname',
                  },
                ],
              },
            },
          },

          replica: {
            replicaCount: args.replicas,

            resources: {
              requests: { cpu: '2' },
              limits: { memory: '4Gi' },
            },

            persistence: {
              storageClass: 'gp3',
              size: '20Gi',
            },

            affinity: {
              podAntiAffinity: {
                requiredDuringSchedulingIgnoredDuringExecution: [
                  {
                    labelSelector: {
                      matchExpressions: [{ key: 'app.kubernetes.io/name', operator: 'In', values: [args.name] }],
                    },
                    topologyKey: 'kubernetes.io/hostname',
                  },
                ],
              },
            },
          },

          metrics: {
            enabled: true,
            serviceMonitor: { enabled: true },
            podMonitor: { enabled: true },
            prometheusRule: { enabled: true },

            resources: {
              requests: { cpu: '500m' },
              limits: { memory: '1Gi' },
            },
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
