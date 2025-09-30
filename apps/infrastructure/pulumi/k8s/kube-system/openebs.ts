import * as k8s from '@pulumi/kubernetes';

new k8s.helm.v4.Chart('openebs', {
  chart: 'openebs',
  namespace: 'kube-system',
  repositoryOpts: {
    repo: 'https://openebs.github.io/openebs',
  },

  // spell-checker:disable
  values: {
    'localpv-provisioner': {
      localpv: { enabled: false },
      rbac: { create: false },
      hostpathClass: { enabled: false },
    },

    'zfs-localpv': {
      zfsNode: {
        encrKeysDir: '/var/openebs/keys',
        allowedTopologyKeys: 'volumes.typie.io/zfs',
      },
    },

    engines: {
      local: {
        lvm: { enabled: false },
        zfs: { enabled: true },
      },

      replicated: {
        mayastor: { enabled: false },
      },
    },

    loki: {
      enabled: false,
    },

    alloy: {
      enabled: false,
    },
  },
  // spell-checker:enable
});

new k8s.storage.v1.StorageClass('zfs', {
  metadata: {
    name: 'zfs',
  },

  provisioner: 'zfs.csi.openebs.io',

  allowVolumeExpansion: true,
  volumeBindingMode: 'WaitForFirstConsumer',
  reclaimPolicy: 'Delete',

  parameters: {
    poolname: 'data',
    fstype: 'zfs',
    recordsize: '8k',
    compression: 'zstd',
    thinprovision: 'no',
  },

  allowedTopologies: [
    {
      matchLabelExpressions: [
        {
          key: 'volumes.typie.io/zfs',
          values: ['true'],
        },
      ],
    },
  ],
});
