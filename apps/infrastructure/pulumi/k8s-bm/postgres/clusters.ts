import * as k8s from '@pulumi/kubernetes';
import * as pulumi from '@pulumi/pulumi';
import * as random from '@pulumi/random';
import { buckets } from '$aws/s3';
import { IAMUserSecret } from '$components';
import { provider } from '$k8s-bm/provider';

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
                Resource: [buckets.backups.arn, pulumi.concat(buckets.backups.arn, '/*')],
              },
            ],
          },
        },
      },
      { parent: this },
    );

    // const backupObjectStore = new k8s.apiextensions.CustomResource(
    //   `${name}-backup`,
    //   {
    //     apiVersion: 'barmancloud.cnpg.io/v1',
    //     kind: 'ObjectStore',

    //     metadata: {
    //       name: pulumi.interpolate`${args.name}-backup`,
    //       namespace: args.namespace,
    //     },

    //     spec: {
    //       retentionPolicy: '30d',
    //       configuration: {
    //         destinationPath: pulumi.interpolate`s3://${buckets.backups.bucket}/postgres/${args.namespace}`,

    //         s3Credentials: {
    //           accessKeyId: {
    //             name: iam.metadata.name,
    //             key: 'AWS_ACCESS_KEY_ID',
    //           },
    //           secretAccessKey: {
    //             name: iam.metadata.name,
    //             key: 'AWS_SECRET_ACCESS_KEY',
    //           },
    //         },

    //         data: {
    //           compression: 'bzip2',
    //         },

    //         wal: {
    //           compression: 'zstd',
    //           maxParallel: 32,
    //         },
    //       },
    //     },
    //   },
    //   { parent: this },
    // );

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
          retentionPolicy: '30d',
          configuration: {
            destinationPath: pulumi.interpolate`s3://${buckets.backups.bucket}/postgres-bm/${args.namespace}`,

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
              maxParallel: 32,
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
                name: credentials.metadata.name,
              },
            },
            // recovery: {
            //   source: 'origin',
            //   secret: {
            //     name: credentials.metadata.name,
            //   },
            // },
          },

          // externalClusters: [
          //   {
          //     name: 'origin',
          //     plugin: {
          //       name: 'barman-cloud.cloudnative-pg.io',
          //       parameters: {
          //         barmanObjectName: backupObjectStore.metadata.name,
          //         serverName: 'db',
          //       },
          //     },
          //   },
          // ],

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
              shared_buffers: '4GB',

              wal_keep_size: '1GB',
              max_wal_size: '8GB',
              checkpoint_timeout: '15min',

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
            storageClass: 'local-ssd',
            size: args.storage.size,
          },

          walStorage: {
            storageClass: 'local-ssd',
            size: args.storage.size,
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
                'external-dns.alpha.kubernetes.io/internal-hostname': args.hostname,
              },
            },
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

// const cluster = new Cluster('db@prod', {
//   name: 'db',
//   namespace: 'prod',

//   instances: 3,

//   hostname: 'db.typie.io',

//   resources: {
//     cpu: '2',
//     memory: '16Gi',
//   },

//   storage: {
//     size: '400Gi',
//   },
// });

const devCluster = new Cluster(
  'db@dev@bm',
  {
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
    },
  },
  { provider },
);

export const outputs = {
  // K8S_POSTGRES_PROD_PASSWORD: cluster.password,
  K8S_BM_POSTGRES_DEV_PASSWORD: devCluster.password,
};
