import * as aws from '@pulumi/aws';
import { securityGroups, subnets } from '$aws/vpc';

export const filesystem = new aws.efs.FileSystem('fs', {
  throughputMode: 'elastic',
  encrypted: true,

  tags: {
    Name: 'fs',
  },
});

new aws.efs.MountTarget('az1', {
  fileSystemId: filesystem.id,
  subnetId: subnets.private.az1.id,
  securityGroups: [securityGroups.internal.id],
});

new aws.efs.MountTarget('az2', {
  fileSystemId: filesystem.id,
  subnetId: subnets.private.az2.id,
  securityGroups: [securityGroups.internal.id],
});
