import * as aws from '@pulumi/aws';
import * as pulumi from '@pulumi/pulumi';
import { workspace } from '$aws/amp';
import { cluster, executionRole } from '$aws/ecs';
import { filesystem } from '$aws/efs';
import { listeners, loadBalancers } from '$aws/elb';
import { zones } from '$aws/route53';
import { securityGroups, subnets, vpc } from '$aws/vpc';

// spell-checker:disable
new aws.ssm.Parameter('/infra/otel-collector/config.yaml', {
  name: '/infra/otel-collector/config.yaml',
  type: 'String',
  value: pulumi.interpolate`
extensions:
  health_check:
  sigv4auth:
    region: ap-northeast-2

receivers:
  otlp:
    protocols:
      grpc:
        endpoint: 0.0.0.0:4317
      http:
        endpoint: 0.0.0.0:4318
  awsecscontainermetrics:
    collection_interval: 10s

processors:
  batch/metrics:
    timeout: 60s
  resourcedetection:
    detectors:
      - env
      - system
      - ecs
      - ec2
  filter:
    metrics:
      include:
        match_type: strict
        metric_names:
          - ecs.task.memory.utilized
          - ecs.task.memory.reserved
          - ecs.task.cpu.utilized
          - ecs.task.cpu.reserved
          - ecs.task.network.rate.rx
          - ecs.task.network.rate.tx
          - ecs.task.storage.read_bytes
          - ecs.task.storage.write_bytes
          - container.duration

exporters:
  prometheusremotewrite:
    endpoint: ${workspace.prometheusEndpoint}api/v1/remote_write
    auth:
      authenticator: sigv4auth
    resource_to_telemetry_conversion:
      enabled: true

service:
  pipelines:
    metrics/application:
      receivers: [otlp]
      processors: [resourcedetection, batch/metrics]
      exporters: [prometheusremotewrite]
    metrics/ecs:
      receivers: [awsecscontainermetrics]
      processors: [filter]
      exporters: [prometheusremotewrite]
  extensions: [health_check, sigv4auth]
  `,
});
// spell-checker:enable

const targetGroup = new aws.lb.TargetGroup('grafana@ecs-tasks', {
  name: 'ecs-tasks-grafana',

  vpcId: vpc.id,
  targetType: 'ip',
  protocol: 'HTTP',
  port: 3000,

  healthCheck: {
    path: '/healthz',
    interval: 10,
    timeout: 5,
    healthyThreshold: 2,
    unhealthyThreshold: 2,
  },

  deregistrationDelay: 5,
});

const rule = new aws.lb.ListenerRule('grafana@ecs-tasks', {
  listenerArn: listeners.private.arn,
  conditions: [{ hostHeader: { values: ['grafana.typie.io'] } }],
  actions: [{ type: 'forward', forward: { targetGroups: [{ arn: targetGroup.arn }] } }],
});

const accessPoint = new aws.efs.AccessPoint('grafana@infra', {
  fileSystemId: filesystem.id,

  rootDirectory: {
    path: '/grafana',
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
    Name: 'grafana',
  },
});

const role = new aws.iam.Role('grafana@ecs-tasks', {
  name: 'grafana@ecs-tasks',
  assumeRolePolicy: aws.iam.assumeRolePolicyForPrincipal({
    Service: 'ecs-tasks.amazonaws.com',
  }),
  managedPolicyArns: [aws.iam.ManagedPolicy.AmazonPrometheusFullAccess, aws.iam.ManagedPolicy.CloudWatchFullAccess],
});

const definition = new aws.ecs.TaskDefinition('grafana', {
  family: 'grafana',

  taskRoleArn: role.arn,
  executionRoleArn: executionRole.arn,

  cpu: '1024',
  memory: '2048',

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
      image: 'grafana/grafana-oss:latest',

      portMappings: [{ containerPort: 3000, hostPort: 3000, protocol: 'tcp' }],

      mountPoints: [
        {
          sourceVolume: 'efs',
          containerPath: '/var/lib/grafana',
        },
      ],

      logConfiguration: {
        logDriver: 'awslogs',
        options: {
          'awslogs-group': '/ecs/grafana',
          'awslogs-region': 'ap-northeast-2',
          'awslogs-stream-prefix': 'ecs',
          'awslogs-create-group': 'true',
        },
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
  'grafana',
  {
    name: 'grafana',
    cluster: cluster.name,

    taskDefinition: definition.arn,
    schedulingStrategy: 'REPLICA',

    desiredCount: 1,

    capacityProviderStrategies: [
      { capacityProvider: 'FARGATE', base: 1 },
      { capacityProvider: 'FARGATE_SPOT', weight: 100 },
    ],

    deploymentMinimumHealthyPercent: 100,
    deploymentMaximumPercent: 200,
    deploymentCircuitBreaker: {
      enable: true,
      rollback: true,
    },

    availabilityZoneRebalancing: 'ENABLED',
    networkConfiguration: {
      subnets: [subnets.private.az1.id, subnets.private.az2.id],
      securityGroups: [securityGroups.internal.id],
    },

    loadBalancers: [
      {
        containerName: 'app',
        containerPort: 3000,
        targetGroupArn: targetGroup.arn,
      },
    ],
  },
  { dependsOn: [rule] },
);

new aws.route53.Record('grafana.typie.io', {
  zoneId: zones.typie_io.zoneId,
  type: 'A',
  name: 'grafana.typie.io',
  aliases: [
    {
      name: loadBalancers.private.dnsName,
      zoneId: loadBalancers.private.zoneId,
      evaluateTargetHealth: true,
    },
  ],
});
