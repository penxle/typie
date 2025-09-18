import * as k8s from '@pulumi/kubernetes';
import { provider } from '$k8s-bm/provider';

new k8s.helm.v4.Chart(
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
        provisionerName: 'rancher.io/local-path',
        defaultClass: true,
      },

      nodePathMap: [{ node: 'DEFAULT_PATH_FOR_NON_LISTED_NODES', paths: ['/data'] }],
    },
  },
  { provider },
);
