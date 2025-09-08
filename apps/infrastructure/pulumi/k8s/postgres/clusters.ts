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
          retentionPolicy: '30d',
          configuration: {
            destinationPath: pulumi.interpolate`s3://${buckets.backups.bucket}/postgres/${args.namespace}`,
            s3Credentials: { inheritFromIAMRole: true },

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
                name: secret.metadata.name,
              },
            },
          },

          primaryUpdateMethod: 'switchover',

          instances: args.instances,

          resources: {
            requests: { cpu: args.resources.cpu },
            limits: { memory: args.resources.memory },
          },

          postgresql: {
            parameters: {
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
            size: args.storage.size,
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

          pgbouncer: {
            poolMode: 'transaction',
            parameters: {
              max_client_conn: '1000',
              default_pool_size: '20',
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
    size: '200Gi',
  },
});

const devCluster = new Cluster('db@dev', {
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
});

export const outputs = {
  K8S_POSTGRES_PROD_PASSWORD: cluster.password,
  K8S_POSTGRES_DEV_PASSWORD: devCluster.password,
};
