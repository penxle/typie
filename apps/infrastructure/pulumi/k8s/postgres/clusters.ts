import * as k8s from '@pulumi/kubernetes';
import * as pulumi from '@pulumi/pulumi';
import * as random from '@pulumi/random';
import { buckets } from '$aws/s3';
import { IAMUserSecret } from '$components';

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

  walSegmentSize: pulumi.Input<number>;

  dbParameters: pulumi.Input<Record<string, string>>;
  poolerParameters: pulumi.Input<Record<string, string>>;
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

    const credentials = new k8s.core.v1.Secret(
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

    const iam = new IAMUserSecret(
      name,
      {
        metadata: {
          name: args.name,
          namespace: args.namespace,
        },
        spec: {
          policy: {
            Version: '2012-10-17',
            Statement: [
              {
                Effect: 'Allow',
                Action: ['s3:ListBucket', 's3:GetObject', 's3:PutObject', 's3:DeleteObject'],
                Resource: [
                  buckets.backups.arn,
                  pulumi.concat(buckets.backups.arn, '/*'),
                  buckets.postgres.arn,
                  pulumi.concat(buckets.postgres.arn, '/*'),
                ],
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
            destinationPath: pulumi.interpolate`s3://${buckets.postgres.bucket}/${args.namespace}`,

            s3Credentials: {
              accessKeyId: {
                name: iam.metadata.name,
                key: 'AWS_ACCESS_KEY_ID',
              },
              secretAccessKey: {
                name: iam.metadata.name,
                key: 'AWS_SECRET_ACCESS_KEY',
              },
            },

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
              walSegmentSize: args.walSegmentSize,

              secret: {
                name: credentials.metadata.name,
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
            parameters: args.dbParameters,
          },

          replicationSlots: {
            highAvailability: {
              enabled: true,
            },
          },

          topologySpreadConstraints: [
            {
              maxSkew: 1,
              topologyKey: 'kubernetes.io/hostname',
              whenUnsatisfiable: 'DoNotSchedule',
              labelSelector: {
                matchLabels: {
                  'cnpg.io/cluster': args.name,
                  'cnpg.io/podRole': 'instance',
                },
              },
            },
          ],

          storage: {
            storageClass: 'zfs',
            size: args.storage.size,
          },

          walStorage: {
            storageClass: 'zfs',
            size: args.storage.walSize,
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
      { parent: this },
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
              topologySpreadConstraints: [
                {
                  maxSkew: 1,
                  topologyKey: 'kubernetes.io/hostname',
                  whenUnsatisfiable: 'DoNotSchedule',
                  labelSelector: {
                    matchLabels: {
                      'cnpg.io/poolerName': `${args.name}-pooler`,
                    },
                  },
                },
              ],
            },
          },

          serviceTemplate: {
            metadata: {
              annotations: {
                'external-dns.typie.io/enabled': 'true',
                'external-dns.alpha.kubernetes.io/internal-hostname': args.hostname,
              },
            },
          },

          pgbouncer: {
            poolMode: 'transaction',
            parameters: args.poolerParameters,
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

const prodCluster = new Cluster('db-prod', {
  name: 'db',
  namespace: 'prod',

  instances: 3,

  hostname: 'db.typie.io',

  resources: {
    cpu: '2',
    memory: '16Gi',
  },

  storage: {
    size: '200Gi',
    walSize: '50Gi',
  },

  walSegmentSize: 256,

  dbParameters: {
    max_connections: '1000',

    wal_buffers: '512MB',
    wal_keep_size: '4GB',
    max_wal_size: '16GB',
    min_wal_size: '4GB',
    wal_writer_delay: '1000ms',
    wal_writer_flush_after: '64MB',

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

  poolerParameters: {
    max_client_conn: '1000',
    min_pool_size: '20',
    default_pool_size: '50',
    reserve_pool_size: '50',
    server_check_delay: '10',
    server_login_retry: '0',
  },
});

const devCluster = new Cluster('db-dev', {
  name: 'db',
  namespace: 'dev',

  instances: 1,

  hostname: 'dev.db.typie.io',

  resources: {
    cpu: '1',
    memory: '2Gi',
  },

  storage: {
    size: '10Gi',
    walSize: '10Gi',
  },

  walSegmentSize: 16,

  dbParameters: {
    max_connections: '100',

    wal_buffers: '4MB',
    wal_keep_size: '1GB',
    max_wal_size: '1GB',
    min_wal_size: '80MB',
    wal_writer_delay: '1000ms',
    wal_writer_flush_after: '1MB',

    checkpoint_timeout: '15min',
    checkpoint_completion_target: '0.9',

    shared_buffers: '256MB',
    effective_cache_size: '1GB',
    work_mem: '4MB',
    maintenance_work_mem: '64MB',

    max_worker_processes: '2',
    max_parallel_workers: '2',
    max_parallel_workers_per_gather: '1',

    default_toast_compression: 'lz4',

    track_activity_query_size: '4096',
    'pg_stat_statements.track': 'ALL',
    'pg_stat_statements.max': '10000',
    'pg_stat_statements.track_utility': '0',
  },

  poolerParameters: {
    max_client_conn: '100',
    min_pool_size: '5',
    default_pool_size: '10',
    reserve_pool_size: '10',
    server_check_delay: '10',
    server_login_retry: '0',
  },
});

export const outputs = {
  K8S_POSTGRES_PROD_PASSWORD: prodCluster.password,
  K8S_POSTGRES_DEV_PASSWORD: devCluster.password,
};
