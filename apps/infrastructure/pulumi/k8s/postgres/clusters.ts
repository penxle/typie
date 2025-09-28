import * as k8s from '@pulumi/kubernetes';
import * as pulumi from '@pulumi/pulumi';
import * as random from '@pulumi/random';
import { buckets } from '$aws/s3';
import { IAMServiceAccount } from '$components';

type ClusterArgs = {
  name: pulumi.Input<string>;
  namespace: pulumi.Input<string>;

  instances: pulumi.Input<number>;

  hostname: pulumi.Input<string>;

  resources: {
    cpu: pulumi.Input<string>;
    memory: pulumi.Input<string>;
  };

  storage: {
    size: pulumi.Input<string>;
    walSize: pulumi.Input<string>;
  };
};

class Cluster extends pulumi.ComponentResource {
  public readonly password: pulumi.Output<string>;

  constructor(name: string, args: ClusterArgs, opts?: pulumi.ComponentResourceOptions) {
    super('typie:postgres:Cluster', name, args, opts);

    const password = new random.RandomPassword(
      `${name}-password`,
      {
        length: 20,
        special: false,
      },
      { parent: this },
    );

    const secret = new k8s.core.v1.Secret(
      `${name}-credentials`,
      {
        metadata: {
          name: pulumi.interpolate`${args.name}-credentials`,
          namespace: args.namespace,
        },
        type: 'kubernetes.io/basic-auth',
        stringData: {
          username: 'app',
          password: password.result,
        },
      },
      { parent: this },
    );

    const serviceAccount = new IAMServiceAccount(
      name,
      {
        metadata: {
          name: args.name,
          namespace: args.namespace,
        },
        spec: {
          serviceAccountName: args.name,
          policy: {
            Version: '2012-10-17',
            Statement: [
              {
                Effect: 'Allow',
                Action: ['s3:ListBucket', 's3:GetObject', 's3:PutObject', 's3:DeleteObject'],
                Resource: [buckets.backups.arn, pulumi.concat(buckets.backups.arn, '/*')],
              },
            ],
          },
        },
      },
      { parent: this },
    );

    const objectStore = new k8s.apiextensions.CustomResource(
      name,
      {
        apiVersion: 'barmancloud.cnpg.io/v1',
        kind: 'ObjectStore',

        metadata: {
          name: args.name,
          namespace: args.namespace,
        },

        spec: {
          retentionPolicy: '7d',
          configuration: {
            destinationPath: pulumi.interpolate`s3://${buckets.backups.bucket}/postgres/${args.namespace}`,
            s3Credentials: { inheritFromIAMRole: true },

            data: {
              compression: 'bzip2',
            },

            wal: {
              compression: 'zstd',
              maxParallel: 8,
            },
          },
        },
      },
      { parent: this },
    );

    const cluster = new k8s.apiextensions.CustomResource(
      name,
      {
        apiVersion: 'postgresql.cnpg.io/v1',
        kind: 'Cluster',

        metadata: {
          name: args.name,
          namespace: args.namespace,
        },

        spec: {
          imageCatalogRef: {
            apiGroup: 'postgresql.cnpg.io',
            kind: 'ClusterImageCatalog',
            name: 'postgresql',
            major: 17,
          },

          bootstrap: {
            initdb: {
              secret: {
                name: secret.metadata.name,
              },
            },
          },

          primaryUpdateMethod: 'switchover',

          instances: args.instances,
          enablePDB: pulumi.output(args.instances).apply((instances) => instances > 1),

          resources: {
            requests: { cpu: args.resources.cpu },
            limits: { memory: args.resources.memory },
          },

          postgresql: {
            parameters: {
              max_connections: '1000',

              wal_buffers: '32MB',
              wal_keep_size: '4GB',
              max_wal_size: '2GB',
              min_wal_size: '512MB',
              wal_writer_delay: '1000ms',
              wal_writer_flush_after: '4MB',

              checkpoint_timeout: '30min',
              checkpoint_completion_target: '0.9',

              shared_buffers: '4GB',
              effective_cache_size: '12GB',
              work_mem: '64MB',
              maintenance_work_mem: '1GB',

              max_worker_processes: '8',
              max_parallel_workers: '8',
              max_parallel_workers_per_gather: '4',

              default_toast_compression: 'lz4',

              track_activity_query_size: '4096',
              'pg_stat_statements.track': 'ALL',
              'pg_stat_statements.max': '10000',
              'pg_stat_statements.track_utility': '0',
            },
          },

          replicationSlots: {
            highAvailability: {
              enabled: true,
            },
          },

          affinity: {
            podAntiAffinityType: 'required',
          },

          // topologySpreadConstraints: [
          //   {
          //     labelSelector: {
          //       matchExpressions: [
          //         { key: 'cnpg.io/cluster', operator: 'In', values: [args.name] },
          //         { key: 'cnpg.io/podRole', operator: 'In', values: ['instance'] },
          //       ],
          //     },
          //     topologyKey: 'topology.kubernetes.io/zone',
          //     maxSkew: 1,
          //     whenUnsatisfiable: 'ScheduleAnyway',
          //   },
          // ],

          storage: {
            storageClass: 'gp3',
            size: args.storage.size,
          },

          walStorage: {
            storageClass: 'gp3',
            size: args.storage.walSize,
          },

          monitoring: {
            enablePodMonitor: true,
          },

          plugins: [
            {
              name: 'barman-cloud.cloudnative-pg.io',
              isWALArchiver: true,
              parameters: {
                barmanObjectName: objectStore.metadata.name,
              },
            },
          ],
        },
      },
      { parent: this, dependsOn: [serviceAccount] },
    );

    new k8s.apiextensions.CustomResource(
      name,
      {
        apiVersion: 'postgresql.cnpg.io/v1',
        kind: 'Pooler',

        metadata: {
          name: `${args.name}-pooler`,
          namespace: args.namespace,
        },

        spec: {
          cluster: {
            name: cluster.metadata.name,
          },

          instances: args.instances,
          type: 'rw',

          template: {
            spec: {
              containers: [],
              affinity: {
                podAntiAffinity: {
                  requiredDuringSchedulingIgnoredDuringExecution: [
                    {
                      labelSelector: {
                        matchExpressions: [{ key: 'cnpg.io/poolerName', operator: 'In', values: [`${args.name}-pooler`] }],
                      },
                      topologyKey: 'kubernetes.io/hostname',
                    },
                  ],
                },
              },
              // topologySpreadConstraints: [
              //   {
              //     labelSelector: {
              //       matchExpressions: [{ key: 'cnpg.io/poolerName', operator: 'In', values: [`${args.name}-pooler`] }],
              //     },
              //     topologyKey: 'topology.kubernetes.io/zone',
              //     maxSkew: 1,
              //     whenUnsatisfiable: 'ScheduleAnyway',
              //   },
              // ],
            },
          },

          serviceTemplate: {
            metadata: {
              annotations: {
                'external-dns.alpha.kubernetes.io/hostname': args.hostname,
                'tailscale.com/proxy-group': 'ingress',
              },
            },
            spec: {
              type: 'LoadBalancer',
              loadBalancerClass: 'tailscale',
            },
          },

          monitoring: {
            enablePodMonitor: true,
          },

          pgbouncer: {
            poolMode: 'transaction',
            parameters: {
              max_client_conn: '1000',
              min_pool_size: '20',
              default_pool_size: '50',
              reserve_pool_size: '50',
              server_check_delay: '10',
              server_login_retry: '0',
            },
          },
        },
      },
      { parent: this },
    );

    new k8s.apiextensions.CustomResource(
      name,
      {
        apiVersion: 'postgresql.cnpg.io/v1',
        kind: 'ScheduledBackup',

        metadata: {
          name: args.name,
          namespace: args.namespace,
        },

        spec: {
          schedule: '0 0 19 * * *',
          backupOwnerReference: 'self',

          cluster: {
            name: cluster.metadata.name,
          },

          method: 'plugin',
          pluginConfiguration: {
            name: 'barman-cloud.cloudnative-pg.io',
          },
        },
      },
      { parent: this },
    );

    this.password = password.result;
  }
}

const cluster = new Cluster('db@prod', {
  name: 'db',
  namespace: 'prod',

  instances: 3,

  hostname: 'db.typie.io',

  resources: {
    cpu: '2',
    memory: '16Gi',
  },

  storage: {
    size: '400Gi',
    walSize: '400Gi',
  },
});

export const outputs = {
  K8S_POSTGRES_PROD_PASSWORD: cluster.password,
};
