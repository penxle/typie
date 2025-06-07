import * as aws from '@pulumi/aws';

export const executionRole = new aws.iam.Role('execution@ecs', {
  name: 'execution@ecs',
  assumeRolePolicy: aws.iam.assumeRolePolicyForPrincipal({
    Service: 'ecs-tasks.amazonaws.com',
  }),
  managedPolicyArns: [aws.iam.ManagedPolicy.AmazonECSTaskExecutionRolePolicy],
});

new aws.iam.RolePolicy('execution@ecs', {
  name: 'execution@ecs',
  role: executionRole.name,
  policy: {
    Version: '2012-10-17',
    Statement: [
      {
        Effect: 'Allow',
        Action: 'ssm:GetParameters',
        Resource: '*',
      },
      {
        Effect: 'Allow',
        Action: 'logs:CreateLogGroup',
        Resource: '*',
      },
    ],
  },
});

export const cluster = new aws.ecs.Cluster('typie', {
  name: 'typie',

  settings: [{ name: 'containerInsights', value: 'disabled' }],
});

new aws.ecs.ClusterCapacityProviders('typie', {
  clusterName: cluster.name,

  capacityProviders: ['FARGATE', 'FARGATE_SPOT'],
  defaultCapacityProviderStrategies: [
    { capacityProvider: 'FARGATE', base: 1 },
    { capacityProvider: 'FARGATE_SPOT', weight: 100 },
  ],
});

export const outputs = {
  AWS_ECS_EXECUTION_ROLE_ARN: executionRole.arn,
};
