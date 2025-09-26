import * as k8s from '@pulumi/kubernetes';
import * as pulumi from '@pulumi/pulumi';
import * as random from '@pulumi/random';
import { provider } from '$k8s-bm/provider';

type MeilisearchArgs = {
  name: pulumi.Input<string>;
  namespace: pulumi.Input<string>;

  hostname: pulumi.Input<string>;

  resources: {
    cpu: pulumi.Input<string>;
    memory: pulumi.Input<string>;
  };

  storage: {
    size: pulumi.Input<string>;
  };
};

class Meilisearch extends pulumi.ComponentResource {
  public readonly masterKey: pulumi.Output<string>;

  constructor(name: string, args: MeilisearchArgs, opts?: pulumi.ComponentResourceOptions) {
    super('typie:meilisearch:Meilisearch', name, args, opts);

    const masterKey = new random.RandomPassword(
      `${name}-master-key`,
      {
        length: 20,
        special: false,
      },
      { parent: this },
    );

    const secret = new k8s.core.v1.Secret(
      `${name}-master-key`,
      {
        metadata: {
          name: `${args.name}-master-key`,
          namespace: args.namespace,
        },
        stringData: {
          MEILI_MASTER_KEY: masterKey.result,
        },
      },
      { parent: this },
    );

    new k8s.helm.v4.Chart(
      name,
      {
        name: args.name,

        chart: 'meilisearch',
        namespace: args.namespace,
        repositoryOpts: {
          repo: 'https://meilisearch.github.io/meilisearch-kubernetes',
        },
        values: {
          image: {
            tag: 'v1.22.0',
          },

          resources: {
            requests: { cpu: args.resources.cpu },
            limits: { memory: args.resources.memory },
          },

          auth: {
            existingMasterKeySecret: secret.metadata.name,
          },

          environment: {
            MEILI_ENV: 'production',
          },

          service: {
            annotations: {
              'external-dns.alpha.kubernetes.io/internal-hostname': args.hostname,
            },
          },

          persistence: {
            enabled: true,
            storageClass: 'local-ssd',
            size: args.storage.size,
          },
        },
      },
      { parent: this },
    );

    this.masterKey = masterKey.result;
  }
}

const dev = new Meilisearch(
  'meilisearch@dev@bm',
  {
    name: 'meilisearch',
    namespace: 'dev',

    hostname: 'dev.search.typie.io',

    resources: {
      cpu: '200m',
      memory: '1Gi',
    },

    storage: {
      size: '10Gi',
    },
  },
  { providers: [provider] },
);

// const prod = new Meilisearch('meilisearch@prod', {
//   name: 'meilisearch',
//   namespace: 'prod',

//   hostname: 'search.typie.io',

//   resources: {
//     cpu: '500m',
//     memory: '2Gi',
//   },

//   storage: {
//     size: '20Gi',
//   },
// });

export const outputs = {
  K8S_BM_MEILISEARCH_DEV_MASTER_KEY: dev.masterKey,
  // K8S_BM_MEILISEARCH_PROD_MASTER_KEY: prod.masterKey,
};
