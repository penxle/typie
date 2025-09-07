import * as aws from '@pulumi/aws';
import * as pulumi from '@pulumi/pulumi';
import * as tls from '@pulumi/tls';
import { securityGroups, subnets, vpc } from '$aws/vpc';
import { tailnet } from '$tailscale/key';

const privateKey = new tls.PrivateKey('ec2', {
  algorithm: 'ED25519',
});

export const keyPair = new aws.ec2.KeyPair('ec2', {
  keyName: 'ec2',
  publicKey: privateKey.publicKeyOpenssh,
});

new aws.ec2.Instance(
  'tailnet-vpc-router',
  {
    ami: aws.ec2.getAmiOutput({
      owners: ['amazon'],
      filters: [
        { name: 'name', values: ['al2023-ami-minimal-*'] },
        { name: 'architecture', values: ['arm64'] },
      ],
      mostRecent: true,
    }).id,

    instanceType: 't4g.nano',

    subnetId: subnets.public.az1.id,
    vpcSecurityGroupIds: [securityGroups.tailnet.id],

    sourceDestCheck: false,

    userData: pulumi.interpolate`
#cloud-config
runcmd:
  - [ hostnamectl, hostname, tailnet-vpc-router ]
  - [ sh, -c, echo 'net.ipv4.ip_forward = 1' | tee -a /etc/sysctl.d/99-tailscale.conf ]
  - [ sysctl, -p, /etc/sysctl.d/99-tailscale.conf ]
  - [ sh, -c, curl -fsSL https://tailscale.com/install.sh | sh ]
  - [ tailscale, up, --auth-key=${tailnet.authKey} ]
  - [ tailscale, set, --advertise-routes=${vpc.cidrBlock} ]
  - [ tailscale, set, --hostname=awsvpc-router ]
`.apply((v) => v.trim()),

    tags: { Name: 'tailnet-vpc-router' },
  },
  {
    ignoreChanges: ['ami'],
    replaceOnChanges: ['userData'],
    deleteBeforeReplace: true,
  },
);

export const outputs = {
  AWS_EC2_KEYPAIR_PRIVATE_KEY: privateKey.privateKeyOpenssh,
};
