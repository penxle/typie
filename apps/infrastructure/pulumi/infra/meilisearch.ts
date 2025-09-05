import * as aws from '@pulumi/aws';
import { keyPair } from '$aws/ec2';
import { listeners, loadBalancers } from '$aws/elb';
import { zones } from '$aws/route53';
import { securityGroups, subnets, vpc } from '$aws/vpc';

const meilisearch = new aws.ec2.Instance(
  'meilisearch',
  {
    ami: aws.ec2.getAmiOutput({
      owners: ['amazon'],
      filters: [
        { name: 'name', values: ['al2023-ami-minimal-*'] },
        { name: 'architecture', values: ['arm64'] },
      ],
      mostRecent: true,
    }).id,

    instanceType: 't4g.medium',

    subnetId: subnets.private.az1.id,
    vpcSecurityGroupIds: [securityGroups.internal.id],

    keyName: keyPair.keyName,

    rootBlockDevice: {
      volumeType: 'gp3',
      volumeSize: 20,

      iops: 3000,
      throughput: 125,
    },

    userData: `
#cloud-config
runcmd:
  - [ hostnamectl, hostname, meilisearch ]
`.trim(),

    tags: { Name: 'meilisearch' },
  },
  {
    ignoreChanges: ['ami'],
  },
);

const targetGroup = new aws.lb.TargetGroup('meilisearch', {
  name: 'meilisearch',

  vpcId: vpc.id,
  targetType: 'instance',
  protocol: 'HTTP',
  port: 7700,

  healthCheck: {
    path: '/health',
    interval: 10,
    timeout: 5,
    healthyThreshold: 2,
    unhealthyThreshold: 2,
  },

  deregistrationDelay: 5,
});

new aws.lb.TargetGroupAttachment('meilisearch', {
  targetGroupArn: targetGroup.arn,
  targetId: meilisearch.id,
});

new aws.lb.ListenerRule('meilisearch', {
  listenerArn: listeners.private.arn,
  conditions: [{ hostHeader: { values: ['meilisearch.typie.io'] } }],
  actions: [{ type: 'forward', forward: { targetGroups: [{ arn: targetGroup.arn }] } }],
});

new aws.route53.Record('meilisearch.typie.io', {
  zoneId: zones.typie_io.zoneId,
  name: 'meilisearch.typie.io',
  type: 'A',
  aliases: [
    {
      name: loadBalancers.private.dnsName,
      zoneId: loadBalancers.private.zoneId,
      evaluateTargetHealth: true,
    },
  ],
});
