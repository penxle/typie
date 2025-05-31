import * as aws from '@pulumi/aws';
import * as pulumi from '@pulumi/pulumi';
import * as random from '@pulumi/random';
import { executionRole } from '$aws/ecs';
import { filesystem } from '$aws/efs';
import { listeners, loadBalancers } from '$aws/elb';
import { zones } from '$aws/route53';
import { securityGroups, subnets, vpc } from '$aws/vpc';

export const password = new random.RandomPassword('meilisearch@infra', {
  length: 20,
  special: false,
});

const accessPoint = new aws.efs.AccessPoint('meilisearch@infra', {
  fileSystemId: filesystem.id,

  rootDirectory: {
    path: '/meilisearch',
    creationInfo: {
      ownerUid: 1000,
      ownerGid: 1000,
      permissions: '755',
    },
  },

  posixUser: {
    uid: 1000,
    gid: 1000,
  },

  tags: {
    Name: 'meilisearch',
  },
});

const targetGroup = new aws.lb.TargetGroup('meilisearch@infra', {
  name: 'ecs-tasks-meilisearch',

  vpcId: vpc.id,
  targetType: 'ip',
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

const rule = new aws.lb.ListenerRule('meilisearch@infra', {
  listenerArn: listeners.private.arn,
  conditions: [{ hostHeader: { values: ['meili.typie.io'] } }],
  actions: [{ type: 'forward', forward: { targetGroups: [{ arn: targetGroup.arn }] } }],
});

const definition = new aws.ecs.TaskDefinition('meilisearch', {
  family: 'meilisearch',

  executionRoleArn: executionRole.arn,

  cpu: '2048',
  memory: '4096',

  requiresCompatibilities: ['FARGATE'],
  runtimePlatform: {
    operatingSystemFamily: 'LINUX',
    cpuArchitecture: 'ARM64',
  },

  networkMode: 'awsvpc',

  containerDefinitions: pulumi.jsonStringify([
    {
      essential: true,

      name: 'app',
      image: 'getmeili/meilisearch:v1.14',

      portMappings: [{ containerPort: 7700, hostPort: 7700, protocol: 'tcp' }],

      environment: [
        { name: 'MEILI_ENV', value: 'production' },
        { name: 'MEILI_MASTER_KEY', value: password.result },
      ],

      mountPoints: [
        {
          sourceVolume: 'efs',
          containerPath: '/meili_data',
        },
      ],

      logConfiguration: {
        logDriver: 'awslogs',
        options: {
          'awslogs-group': '/ecs/meilisearch',
          'awslogs-region': 'ap-northeast-2',
          'awslogs-stream-prefix': 'ecs',
          'awslogs-create-group': 'true',
        },
      },

      restartPolicy: {
        enabled: true,
        ignoredExitCodes: [0],
        restartAttemptPeriod: 60,
      },
    },
  ]),

  volumes: [
    {
      name: 'efs',
      efsVolumeConfiguration: {
        fileSystemId: filesystem.id,
        transitEncryption: 'ENABLED',
        authorizationConfig: {
          accessPointId: accessPoint.id,
        },
      },
    },
  ],
});

new aws.ecs.Service(
  'meilisearch',
  {
    name: 'meilisearch',
    cluster: 'typie',

    taskDefinition: definition.arn,
    schedulingStrategy: 'REPLICA',

    desiredCount: 1,

    capacityProviderStrategies: [{ capacityProvider: 'FARGATE', base: 1, weight: 100 }],

    deploymentMinimumHealthyPercent: 0,
    deploymentMaximumPercent: 100,
    deploymentCircuitBreaker: {
      enable: true,
      rollback: true,
    },

    // availabilityZoneRebalancing: 'ENABLED',
    networkConfiguration: {
      subnets: [subnets.private.az1.id, subnets.private.az2.id],
      securityGroups: [securityGroups.internal.id],
    },

    loadBalancers: [
      {
        containerName: 'app',
        containerPort: 7700,
        targetGroupArn: targetGroup.arn,
      },
    ],

    enableEcsManagedTags: true,

    waitForSteadyState: true,
  },
  { dependsOn: [rule] },
);

new aws.route53.Record('meili.typie.io', {
  zoneId: zones.typie_io.zoneId,
  name: 'meili.typie.io',
  type: 'A',
  aliases: [
    {
      name: loadBalancers.private.dnsName,
      zoneId: loadBalancers.private.zoneId,
      evaluateTargetHealth: true,
    },
  ],
});
