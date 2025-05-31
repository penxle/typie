import * as aws from '@pulumi/aws';
import { certificates } from '$aws/acm';
import { securityGroups, subnets } from '$aws/vpc';

const createListeners = (name: string, alb: aws.lb.LoadBalancer) => {
  new aws.lb.Listener(`${name}:80`, {
    loadBalancerArn: alb.arn,

    port: 80,
    protocol: 'HTTP',

    defaultActions: [{ type: 'redirect', redirect: { port: '443', protocol: 'HTTPS', statusCode: 'HTTP_301' } }],
  });

  const listener = new aws.lb.Listener(`${name}:443`, {
    loadBalancerArn: alb.arn,

    port: 443,
    protocol: 'HTTPS',

    defaultActions: [{ type: 'fixed-response', fixedResponse: { contentType: 'text/plain', messageBody: 'Not found', statusCode: '404' } }],

    certificateArn: certificates.typie_io.arn,
    sslPolicy: 'ELBSecurityPolicy-TLS13-1-2-Res-2021-06',
  });

  new aws.lb.ListenerCertificate(`${name}:443[typie_co]`, {
    listenerArn: listener.arn,
    certificateArn: certificates.typie_co.arn,
  });

  new aws.lb.ListenerCertificate(`${name}:443[typie_dev]`, {
    listenerArn: listener.arn,
    certificateArn: certificates.typie_dev.arn,
  });

  new aws.lb.ListenerCertificate(`${name}:443[typie_io]`, {
    listenerArn: listener.arn,
    certificateArn: certificates.typie_io.arn,
  });

  new aws.lb.ListenerCertificate(`${name}:443[typie_net]`, {
    listenerArn: listener.arn,
    certificateArn: certificates.typie_net.arn,
  });

  new aws.lb.ListenerCertificate(`${name}:443[typie_me]`, {
    listenerArn: listener.arn,
    certificateArn: certificates.typie_me.arn,
  });

  return listener;
};

const publicAlb = new aws.lb.LoadBalancer('public', {
  name: 'public',

  ipAddressType: 'ipv4',

  subnets: [subnets.public.az1.id, subnets.public.az2.id],
  securityGroups: [securityGroups.publicWeb.id, securityGroups.internal.id],
});

const privateAlb = new aws.lb.LoadBalancer('private', {
  name: 'private',

  ipAddressType: 'ipv4',
  internal: true,

  subnets: [subnets.private.az1.id, subnets.private.az2.id],
  securityGroups: [securityGroups.internal.id],
});

const publicListener = createListeners('public', publicAlb);
const privateListener = createListeners('private', privateAlb);

export const loadBalancers = { public: publicAlb, private: privateAlb };
export const listeners = { public: publicListener, private: privateListener };

export const outputs = {
  AWS_ELB_PUBLIC_DNS_NAME: publicAlb.dnsName,
  AWS_ELB_PRIVATE_DNS_NAME: privateAlb.dnsName,

  AWS_ELB_PUBLIC_ZONE_ID: publicAlb.zoneId,
  AWS_ELB_PRIVATE_ZONE_ID: privateAlb.zoneId,

  AWS_ELB_PUBLIC_LISTENER_ARN: publicListener.arn,
  AWS_ELB_PRIVATE_LISTENER_ARN: privateListener.arn,
};
