import * as aws from '@pulumi/aws';
import * as command from '@pulumi/command';
import * as pulumi from '@pulumi/pulumi';

type ServiceArgs = {
  name: pulumi.Input<string>;

  image: {
    name: pulumi.Input<string>;
    version: pulumi.Input<string>;
  };

  resources: {
    cpu: pulumi.Input<string>;
    memory: pulumi.Input<string>;
  };

  autoscale?: {
    minCount: pulumi.Input<number>;
    maxCount: pulumi.Input<number>;
    averageCpuUtilization: pulumi.Input<number>;
  };

  iam?: {
    policy: pulumi.Input<aws.iam.PolicyDocument>;
  };

  env?: {
    entries: pulumi.Input<pulumi.Input<string>[]>;
  };

  domains: pulumi.Input<pulumi.Input<string>[]>;
};

export class Service extends pulumi.ComponentResource {
  constructor(name: string, args: ServiceArgs, opts?: pulumi.ComponentResourceOptions) {
    super('typie:index:Service', name, {}, opts);

    const ref = new pulumi.StackReference('typie/infrastructure/base', {}, { parent: this });

    const project = pulumi.getProject();
    const stack = pulumi.getStack();

    const serviceName = pulumi.interpolate`${stack}-${args.name}`;

    const role = new aws.iam.Role(
      `${name}@ecs-tasks`,
      {
        name: pulumi.interpolate`${serviceName}@ecs-tasks`,
        assumeRolePolicy: aws.iam.assumeRolePolicyForPrincipal({
          Service: 'ecs-tasks.amazonaws.com',
        }),
        managedPolicyArns: [aws.iam.ManagedPolicy.AmazonSSMReadOnlyAccess, aws.iam.ManagedPolicy.CloudWatchLogsFullAccess],
      },
      { parent: this },
    );

    if (args.iam) {
      new aws.iam.RolePolicy(`${name}@ecs-tasks`, { role: role.name, policy: args.iam.policy }, { parent: this });
    }

    // Blue Target Group
    const targetGroupBlue = new aws.lb.TargetGroup(
      `${name}-blue@ecs-tasks`,
      {
        name: pulumi.interpolate`ecs-tasks-${serviceName}-blue`,

        vpcId: ref.requireOutput('AWS_VPC_ID'),
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
      },
      { parent: this },
    );

    // Green Target Group
    const targetGroupGreen = new aws.lb.TargetGroup(
      `${name}-green@ecs-tasks`,
      {
        name: pulumi.interpolate`ecs-tasks-${serviceName}-green`,

        vpcId: ref.requireOutput('AWS_VPC_ID'),
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
      },
      { parent: this },
    );

    const rule = new aws.lb.ListenerRule(
      `${name}@ecs-tasks`,
      {
        listenerArn: ref.requireOutput('AWS_ELB_PUBLIC_LISTENER_ARN'),
        conditions: [{ hostHeader: { values: args.domains } }],
        actions: [{ type: 'forward', forward: { targetGroups: [{ arn: targetGroupBlue.arn }] } }],
      },
      { parent: this },
    );

    const definition = new aws.ecs.TaskDefinition(
      name,
      {
        family: serviceName,

        taskRoleArn: role?.arn,
        executionRoleArn: ref.requireOutput('AWS_ECS_EXECUTION_ROLE_ARN'),

        cpu: args.resources.cpu,
        memory: args.resources.memory,

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
            image: pulumi.interpolate`${args.image.name}:${args.image.version}`,

            cpu: pulumi.output(args.resources.cpu).apply((cpu) => Number.parseInt(cpu)),
            memory: pulumi.output(args.resources.memory).apply((memory) => Number.parseInt(memory)),

            portMappings: [{ containerPort: 3000, hostPort: 3000, protocol: 'tcp' }],

            environment: [
              { name: 'LISTEN_PORT', value: '3000' },
              { name: 'PUBLIC_PULUMI_PROJECT', value: project },
              { name: 'PUBLIC_PULUMI_STACK', value: stack },
            ],

            secrets: pulumi.output(args.env?.entries).apply((entries) =>
              entries?.map((entry) => ({
                name: entry,
                valueFrom: pulumi.interpolate`/apps/${name}/${stack}/${entry}`,
              })),
            ),

            logConfiguration: {
              logDriver: 'awslogs',
              options: {
                'awslogs-group': pulumi.interpolate`/ecs/${args.name}/${stack}`,
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
          {
            essential: true,

            name: 'datadog-agent',
            image: 'public.ecr.aws/datadog/agent:latest',

            environment: [
              { name: 'ECS_FARGATE', value: 'true' },
              { name: 'DD_SITE', value: 'ap1.datadoghq.com' },
            ],

            secrets: [{ name: 'DD_API_KEY', valueFrom: '/datadog/api-key' }],
          },
        ]),
      },
      { parent: this },
    );

    const service = new aws.ecs.Service(
      name,
      {
        name: serviceName,
        cluster: 'typie',

        taskDefinition: definition.arn,
        schedulingStrategy: 'REPLICA',

        desiredCount: stack === 'dev' ? 1 : args.autoscale?.minCount,

        capacityProviderStrategies: [
          { capacityProvider: 'FARGATE', base: 0 },
          { capacityProvider: 'FARGATE_SPOT', weight: 100 },
        ],

        deploymentController: {
          type: 'CODE_DEPLOY',
        },

        availabilityZoneRebalancing: 'ENABLED',
        networkConfiguration: {
          subnets: [ref.requireOutput('AWS_VPC_SUBNET_PRIVATE_AZ1_ID'), ref.requireOutput('AWS_VPC_SUBNET_PRIVATE_AZ2_ID')],
          securityGroups: [ref.requireOutput('AWS_VPC_SECURITY_GROUP_INTERNAL_ID')],
        },

        loadBalancers: [
          {
            containerName: 'app',
            containerPort: 3000,
            targetGroupArn: targetGroupBlue.arn,
          },
        ],
      },
      { parent: this, dependsOn: [rule], ignoreChanges: ['desiredCount', 'taskDefinition', 'loadBalancers'] },
    );

    const app = new aws.codedeploy.Application(
      `${name}@ecs-tasks`,
      {
        name: serviceName,
        computePlatform: 'ECS',
      },
      { parent: this },
    );

    const deploymentGroup = new aws.codedeploy.DeploymentGroup(
      `${name}@ecs-tasks`,
      {
        deploymentGroupName: serviceName,
        appName: app.name,
        serviceRoleArn: ref.requireOutput('AWS_ECS_CODEDEPLOY_ROLE_ARN'),

        ecsService: {
          clusterName: 'typie',
          serviceName: service.name,
        },

        deploymentConfigName: 'CodeDeployDefault.ECSAllAtOnce',

        deploymentStyle: {
          deploymentType: 'BLUE_GREEN',
          deploymentOption: 'WITH_TRAFFIC_CONTROL',
        },

        blueGreenDeploymentConfig: {
          deploymentReadyOption: {
            actionOnTimeout: 'CONTINUE_DEPLOYMENT',
          },

          terminateBlueInstancesOnDeploymentSuccess: {
            action: 'TERMINATE',
            terminationWaitTimeInMinutes: 5,
          },
        },

        loadBalancerInfo: {
          targetGroupPairInfo: {
            prodTrafficRoute: {
              listenerArns: [ref.requireOutput('AWS_ELB_PUBLIC_LISTENER_ARN')],
            },

            targetGroups: [
              {
                name: targetGroupBlue.name,
              },
              {
                name: targetGroupGreen.name,
              },
            ],
          },
        },

        autoRollbackConfiguration: {
          enabled: true,
          events: ['DEPLOYMENT_FAILURE', 'DEPLOYMENT_STOP_ON_ALARM'],
        },
      },
      { parent: this },
    );

    const deploymentInput = pulumi.jsonStringify({
      applicationName: app.name,
      deploymentGroupName: serviceName,
      description: 'Deployment triggered by Pulumi',
      revision: {
        revisionType: 'AppSpecContent',
        appSpecContent: {
          content: pulumi.jsonStringify({
            version: 0,
            Resources: [
              {
                TargetService: {
                  Type: 'AWS::ECS::Service',
                  Properties: {
                    TaskDefinition: definition.arn,
                    LoadBalancerInfo: {
                      ContainerName: 'app',
                      ContainerPort: 3000,
                    },
                  },
                },
              },
            ],
          }),
        },
      },
    });

    new command.local.Command(
      `${name}@codedeploy`,
      {
        create: pulumi.interpolate`aws deploy create-deployment --cli-input-json '${deploymentInput}' --output json`,
        triggers: [definition.arn],
      },
      { parent: this, dependsOn: [deploymentGroup] },
    );

    if (args.autoscale && stack === 'prod') {
      const target = new aws.appautoscaling.Target(
        `${name}@ecs-tasks`,
        {
          resourceId: pulumi.interpolate`service/typie/${serviceName}`,
          serviceNamespace: 'ecs',
          scalableDimension: 'ecs:service:DesiredCount',

          minCapacity: args.autoscale.minCount,
          maxCapacity: args.autoscale.maxCount,
        },
        {
          parent: this,
          dependsOn: [service],
        },
      );

      new aws.appautoscaling.Policy(
        `${name}@ecs-tasks`,
        {
          name: pulumi.interpolate`${serviceName}@ecs-tasks`,

          resourceId: target.resourceId,
          scalableDimension: target.scalableDimension,
          serviceNamespace: target.serviceNamespace,

          policyType: 'TargetTrackingScaling',
          targetTrackingScalingPolicyConfiguration: {
            targetValue: args.autoscale.averageCpuUtilization,
            predefinedMetricSpecification: {
              predefinedMetricType: 'ECSServiceAverageCPUUtilization',
            },
          },
        },
        { parent: this },
      );
    }
  }
}
