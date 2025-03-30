import * as k8s from '@pulumi/kubernetes';
import * as random from '@pulumi/random';
import { namespace } from '$k8s/infra';

const password = new random.RandomPassword('meilisearch@infra', {
  length: 20,
  special: false,
});

const pvc = new k8s.core.v1.PersistentVolumeClaim('meilisearch@infra', {
  metadata: {
    name: 'meilisearch',
    namespace: namespace.metadata.name,
  },

  spec: {
    storageClassName: 'ebs',
    accessModes: ['ReadWriteOnce'],
    resources: {
      requests: {
        storage: '20Gi',
      },
    },
  },
});

new k8s.helm.v4.Chart('meilisearch', {
  chart: 'meilisearch',
  version: '0.12.0',
  namespace: namespace.metadata.name,
  repositoryOpts: {
    repo: 'https://meilisearch.github.io/meilisearch-kubernetes',
  },

  values: {
    environment: {
      MEILI_ENV: 'production',
      MEILI_MASTER_KEY: password.result,
    },

    persistence: {
      enabled: true,
      existingClaim: pvc.metadata.name,
    },

    service: {
      type: 'NodePort',
    },

    ingress: {
      enabled: true,
      className: 'alb',
      annotations: {
        'alb.ingress.kubernetes.io/group.name': 'private-alb',
        'alb.ingress.kubernetes.io/group.order': '10',
        'alb.ingress.kubernetes.io/listen-ports': JSON.stringify([{ HTTPS: 443 }]),
        'alb.ingress.kubernetes.io/healthcheck-path': '/health',
      },
      hosts: ['meili.typie.io'],
    },
  },
});

export const outputs = {
  K8S_INFRA_MEILISEARCH_PASSWORD: password.result,
};
