import * as aws from '@pulumi/aws';
import * as k8s from '@pulumi/kubernetes';
import * as pulumi from '@pulumi/pulumi';

type IAMUserSecretArgs = {
  metadata: {
    name: pulumi.Input<string>;
    namespace: pulumi.Input<string>;
  };
  spec: {
    policy: pulumi.Input<aws.iam.PolicyDocument>;
  };
};

type IAMUserSecretOutputMetadata = {
  name: string;
  namespace: string;
};

export class IAMUserSecret extends pulumi.ComponentResource {
  public readonly metadata: pulumi.Output<IAMUserSecretOutputMetadata>;

  constructor(name: string, args: IAMUserSecretArgs, opts?: pulumi.ComponentResourceOptions) {
    super('typie:index:IAMUserSecret', name, {}, opts);

    const user = new aws.iam.User(
      `${name}@k8s`,
      {
        name: pulumi.interpolate`${args.metadata.name}+${args.metadata.namespace}@k8s`,
      },
      { parent: this },
    );

    new aws.iam.UserPolicy(
      `${name}@k8s`,
      {
        user: user.name,
        policy: args.spec.policy,
      },
      { parent: this },
    );

    const accessKey = new aws.iam.AccessKey(
      `${name}@k8s`,
      {
        user: user.name,
      },
      { parent: this },
    );

    const secret = new k8s.core.v1.Secret(
      name,
      {
        metadata: {
          name: args.metadata.name,
          namespace: args.metadata.namespace,
        },
        stringData: {
          AWS_REGION: 'ap-northeast-2',
          AWS_ACCESS_KEY_ID: accessKey.id,
          AWS_SECRET_ACCESS_KEY: accessKey.secret,
        },
      },
      { parent: this },
    );

    this.metadata = pulumi.output({
      name: secret.metadata.name,
      namespace: secret.metadata.namespace,
    });
  }
}
