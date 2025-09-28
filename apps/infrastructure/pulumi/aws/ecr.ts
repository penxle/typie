import * as aws from '@pulumi/aws';
import * as pulumi from '@pulumi/pulumi';

type RepositoryArgs = {
  name: pulumi.Input<string>;
};

class Repository extends pulumi.ComponentResource {
  constructor(name: string, args: RepositoryArgs, opts?: pulumi.ComponentResourceOptions) {
    super('typie:index:Repository', name, args, opts);

    const repository = new aws.ecr.Repository(
      name,
      {
        name: args.name,
        forceDelete: true,
      },
      { parent: this },
    );

    new aws.ecr.LifecyclePolicy(
      name,
      {
        repository: repository.name,
        policy: {
          rules: [
            {
              rulePriority: 1,
              selection: {
                tagStatus: 'any',
                countType: 'imageCountMoreThan',
                countNumber: 5,
              },
              action: {
                type: 'expire',
              },
            },
          ],
        },
      },
      { parent: this },
    );
  }
}

const createRepository = (name: string) => {
  return new Repository(name, { name });
};

createRepository('api');
createRepository('website');

const user = new aws.iam.User('ecr-credential-provider@k8s', {
  name: 'ecr-credential-provider@k8s',
});

new aws.iam.UserPolicyAttachment('ecr-credential-provider@k8s', {
  user: user.name,
  policyArn: aws.iam.ManagedPolicy.AmazonEC2ContainerRegistryReadOnly,
});

const accessKey = new aws.iam.AccessKey('ecr-credential-provider@k8s', {
  user: user.name,
});

export const outputs = {
  ECR_CREDENTIAL_PROVIDER_ACCESS_KEY_ID: accessKey.id,
  ECR_CREDENTIAL_PROVIDER_SECRET_ACCESS_KEY: accessKey.secret,
};
