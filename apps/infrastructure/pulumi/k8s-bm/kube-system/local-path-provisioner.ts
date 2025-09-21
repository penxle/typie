import * as k8s from '@pulumi/kubernetes';
import { provider } from '$k8s-bm/provider';

const chart = new k8s.helm.v4.Chart(
  'local-path-provisioner@bm',
  {
    name: 'local-path-provisioner',

    chart: 'local-path-provisioner',
    namespace: 'kube-system',
    repositoryOpts: {
      repo: 'https://charts.containeroo.ch',
    },

    values: {
      storageClass: {
        create: false,
        provisionerName: 'rancher.io/local-path',
      },

      nodePathMap: [{ node: 'DEFAULT_PATH_FOR_NON_LISTED_NODES', paths: ['/var/mnt/data'] }],
    },
  },
  { provider },
);

new k8s.storage.v1.StorageClass(
  'local-ssd@bm',
  {
    metadata: {
      name: 'local-ssd',
      annotations: {
        'storageclass.kubernetes.io/is-default-class': 'true',
        defaultVolumeType: 'hostPath',
      },
    },

    provisioner: 'rancher.io/local-path',
    volumeBindingMode: 'WaitForFirstConsumer',
    reclaimPolicy: 'Delete',
    allowVolumeExpansion: true,

    allowedTopologies: [{ matchLabelExpressions: [{ key: 'volumes.typie.io/attach', values: ['true'] }] }],
  },
  { provider, dependsOn: [chart] },
);
