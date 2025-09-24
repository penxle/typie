import * as aws from '@pulumi/aws';
import * as pulumi from '@pulumi/pulumi';
import { buckets } from '$aws/s3';
import { configurationSet, emailIdentity } from '$aws/ses';

const team = new aws.iam.Group('team', {
  name: 'team',
});

new aws.iam.GroupPolicy('team', {
  group: team.name,
  name: 'team',
  policy: {
    Version: '2012-10-17',
    Statement: [
      {
        Effect: 'Allow',
        Action: ['secretsmanager:GetSecretValue', 'secretsmanager:DescribeSecret'],
        Resource: ['arn:aws:secretsmanager:*:*:secret:/apps/*/local-*', 'arn:aws:secretsmanager:*:*:secret:/apps/*/dev-*'],
      },
      {
        Effect: 'Allow',
        Action: ['s3:GetObject', 's3:PutObject'],
        Resource: [pulumi.concat(buckets.uploads.arn, '/*')],
      },
      {
        Effect: 'Allow',
        Action: ['s3:GetObject', 's3:PutObject', 's3:GetObjectTagging', 's3:PutObjectTagging'],
        Resource: [pulumi.concat(buckets.usercontents.arn, '/*')],
      },
      {
        Effect: 'Allow',
        Action: ['s3:GetObject', 's3:PutObject', 's3:GetObjectTagging', 's3:PutObjectTagging'],
        Resource: [pulumi.concat(buckets.misc.arn, '/*')],
      },
      {
        Effect: 'Allow',
        Action: ['ses:SendEmail'],
        Resource: [emailIdentity.arn, configurationSet.arn],
        Condition: {
          StringEquals: {
            'ses:FromAddress': 'hello@typie.co',
          },
        },
      },
      {
        Effect: 'Allow',
        Action: ['ce:GetCostAndUsage'],
        Resource: '*',
      },
      {
        Effect: 'Allow',
        Action: [
          'iam:GetUser',
          'iam:ListAccessKeys',
          'iam:ListGroupsForUser',
          'iam:ChangePassword',
          'iam:CreateAccessKey',
          'iam:DeleteAccessKey',
          'iam:UpdateAccessKey',
          'iam:GetAccessKeyLastUsed',
          'iam:CreateVirtualMFADevice',
          'iam:DeleteVirtualMFADevice',
          'iam:EnableMFADevice',
          'iam:DeactivateMFADevice',
          'iam:ListMFADevices',
          'iam:ResyncMFADevice',
        ],
        Resource: 'arn:aws:iam::*:user/${aws:username}',
      },
    ],
  },
});

const githubActionsOidcProvider = new aws.iam.OpenIdConnectProvider('actions@github', {
  url: 'https://token.actions.githubusercontent.com',
  clientIdLists: ['sts.amazonaws.com'],
  thumbprintLists: ['ffffffffffffffffffffffffffffffffffffffff'],
});

const githubActionsRole = new aws.iam.Role('actions@github', {
  name: 'actions@github',
  assumeRolePolicy: {
    Version: '2012-10-17',
    Statement: [
      {
        Effect: 'Allow',
        Principal: { Federated: githubActionsOidcProvider.arn },
        Action: 'sts:AssumeRoleWithWebIdentity',
        Condition: {
          StringLike: {
            'token.actions.githubusercontent.com:sub': 'repo:penxle/*',
          },
          StringEquals: {
            'token.actions.githubusercontent.com:aud': 'sts.amazonaws.com',
          },
        },
      },
    ],
  },
});

new aws.iam.RolePolicy('actions@github', {
  role: githubActionsRole.name,
  policy: {
    Version: '2012-10-17',
    Statement: [
      {
        Effect: 'Allow',
        Action: '*',
        Resource: '*',
      },
    ],
  },
});

export const roles = {
  actions: githubActionsRole,
};
