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
                countNumber: 50,
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
