import * as k8s from '@pulumi/kubernetes';

new k8s.storage.v1.StorageClass('ebs', {
  metadata: {
    name: 'ebs',
  },

  provisioner: 'ebs.csi.aws.com',

  allowVolumeExpansion: true,
  volumeBindingMode: 'WaitForFirstConsumer',
  reclaimPolicy: 'Delete',

  parameters: {
    // spell-checker:disable-next-line
    'csi.storage.k8s.io/fstype': 'xfs',
    type: 'gp3',

    iops: '3000',
    throughput: '125',

    encrypted: 'true',
  },
});
