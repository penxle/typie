import * as k8s from '@pulumi/kubernetes';
import * as pulumi from '@pulumi/pulumi';
import * as random from '@pulumi/random';

type ClusterArgs = {
  name: string;
  namespace: string;

  replicas: number;

  resources: {
    cpu: string;
    memory: string;
  };

  storage: {
    size: string;
  };

  hostname: string;
};

class Cluster extends pulumi.ComponentResource {
  public readonly password: pulumi.Output<string>;

  constructor(name: string, args: ClusterArgs, opts?: pulumi.ComponentResourceOptions) {
    super('typie:rabbitmq:Cluster', name, args, opts);

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
        stringData: {
          username: 'app',
          password: password.result,
        },
      },
      { parent: this },
    );

    const cluster = new k8s.apiextensions.CustomResource(
      name,
      {
        apiVersion: 'rabbitmq.com/v1beta1',
        kind: 'RabbitmqCluster',

        metadata: {
          name: args.name,
          namespace: args.namespace,
        },

        spec: {
          replicas: args.replicas,

          resources: {
            requests: { cpu: args.resources.cpu },
            limits: { memory: args.resources.memory },
          },

          persistence: {
            storageClassName: 'gp3',
            storage: args.storage.size,
          },

          affinity: {
            podAntiAffinity: {
              requiredDuringSchedulingIgnoredDuringExecution: [
                {
                  labelSelector: { matchExpressions: [{ key: 'app.kubernetes.io/name', operator: 'In', values: [args.name] }] },
                  topologyKey: 'kubernetes.io/hostname',
                },
              ],
            },
          },

          override: {
            service: {
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
          },

          autoEnableAllFeatureFlags: true,
        },
      },
      { parent: this },
    );

    const user = new k8s.apiextensions.CustomResource(
      name,
      {
        apiVersion: 'rabbitmq.com/v1beta1',
        kind: 'User',

        metadata: {
          name: args.name,
          namespace: args.namespace,
        },

        spec: {
          rabbitmqClusterReference: {
            name: cluster.metadata.name,
          },

          importCredentialsSecret: {
            name: secret.metadata.name,
          },

          tags: ['management'],
        },
      },
      { parent: this },
    );

    new k8s.apiextensions.CustomResource(
      name,
      {
        apiVersion: 'rabbitmq.com/v1beta1',
        kind: 'Permission',

        metadata: {
          name: args.name,
          namespace: args.namespace,
        },

        spec: {
          rabbitmqClusterReference: {
            name: cluster.metadata.name,
          },

          userReference: {
            name: user.metadata.name,
          },

          vhost: '/',
          permissions: {
            read: '.*',
            write: '.*',
            configure: '.*',
          },
        },
      },
      { parent: this },
    );

    this.password = password.result;
  }
}

const devCluster = new Cluster('mq@dev', {
  name: 'mq',
  namespace: 'dev',

  replicas: 1,

  resources: {
    cpu: '2',
    memory: '4Gi',
  },

  storage: {
    size: '10Gi',
  },

  hostname: 'dev.mq.typie.io',
});

const cluster = new Cluster('mq@prod', {
  name: 'mq',
  namespace: 'prod',

  replicas: 3,

  resources: {
    cpu: '2',
    memory: '4Gi',
  },

  storage: {
    size: '20Gi',
  },

  hostname: 'mq.typie.io',
});

export const outputs = {
  K8S_RABBITMQ_DEV_PASSWORD: devCluster.password,
  K8S_RABBITMQ_PROD_PASSWORD: cluster.password,
};
