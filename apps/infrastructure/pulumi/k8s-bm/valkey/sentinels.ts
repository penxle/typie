import * as k8s from '@pulumi/kubernetes';
import * as pulumi from '@pulumi/pulumi';
import dedent from 'dedent';
import { provider } from '$k8s-bm/provider';

type SentinelArgs = {
  name: pulumi.Input<string>;
  namespace: pulumi.Input<string>;

  replicas: pulumi.Input<number>;
  hostname: pulumi.Input<string>;

  maxmemory: pulumi.Input<string>;

  resources: {
    cpu: pulumi.Input<string>;
    memory: pulumi.Input<string>;
  };

  storage: {
    size: pulumi.Input<string>;
  };
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

          commonConfiguration: pulumi.interpolate`
            maxmemory ${args.maxmemory}
            maxmemory-policy noeviction

            appendonly yes
            appendfsync everysec

            save ""
          `.apply((v) => dedent(v)),

          useHostnames: false,

          image: {
            repository: 'bitnamilegacy/valkey',
          },

          auth: {
            enabled: false,
          },

          sentinel: {
            enabled: true,
            primarySet: 'primary',

            image: {
              repository: 'bitnamilegacy/valkey-sentinel',
            },

            resources: {
              requests: { cpu: '100m' },
              limits: { memory: '256Mi' },
            },

            service: {
              annotations: {
                'external-dns.typie.io/enabled': 'true',
                'external-dns.alpha.kubernetes.io/internal-hostname': args.hostname,
              },
            },
          },

          primary: {
            replicaCount: 1,

            resources: {
              requests: { cpu: args.resources.cpu },
              limits: { memory: args.resources.memory },
            },

            persistence: {
              storageClass: 'zfs',
              size: args.storage.size,
            },

            topologySpreadConstraints: [
              {
                maxSkew: 1,
                topologyKey: 'kubernetes.io/hostname',
                whenUnsatisfiable: 'DoNotSchedule',
                labelSelector: {
                  matchLabels: {
                    'app.kubernetes.io/name': args.name,
                  },
                },
              },
            ],
          },

          replica: {
            replicaCount: args.replicas,

            resources: {
              requests: { cpu: args.resources.cpu },
              limits: { memory: args.resources.memory },
            },

            persistence: {
              storageClass: 'zfs',
              size: args.storage.size,
            },

            topologySpreadConstraints: [
              {
                maxSkew: 1,
                topologyKey: 'kubernetes.io/hostname',
                whenUnsatisfiable: 'DoNotSchedule',
                labelSelector: {
                  matchLabels: {
                    'app.kubernetes.io/name': args.name,
                  },
                },
              },
            ],
          },

          // metrics: {
          //   enabled: true,
          //   serviceMonitor: { enabled: true },
          //   podMonitor: { enabled: true },
          //   prometheusRule: { enabled: true },

          //   image: {
          //     repository: 'bitnamilegacy/redis-exporter',
          //   },

          //   resources: {
          //     requests: { cpu: '100m' },
          //     limits: { memory: '256Mi' },
          //   },
          // },

          volumePermissions: {
            image: {
              repository: 'bitnamilegacy/os-shell',
            },
          },

          kubectl: {
            image: {
              repository: 'bitnamilegacy/kubectl',
            },
          },
        },
      },
      { parent: this },
    );
  }
}

new Sentinel(
  'redis@dev@bm',
  {
    name: 'valkey',
    namespace: 'dev',

    replicas: 1,
    hostname: 'dev.redis.typie.io',

    maxmemory: '256mb',

    resources: {
      cpu: '200m',
      memory: '512Mi',
    },

    storage: {
      size: '10Gi',
    },
  },
  { provider },
);

// new Sentinel('redis@prod', {
//   name: 'valkey',
//   namespace: 'prod',

//   replicas: 3,
//   hostname: 'redis.typie.io',

//   maxmemory: '512mb',

//   resources: {
//     cpu: '500m',
//     memory: '1Gi',
//   },
// });
