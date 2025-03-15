import * as glitter from '@glitter/pulumi';
import * as pulumi from '@pulumi/pulumi';

const config = new pulumi.Config('glitter');

new glitter.Service('website', {
  name: 'website',

  image: {
    name: '509399603331.dkr.ecr.ap-northeast-2.amazonaws.com/glitter',
    digest: config.require('digest'),
    command: ['bun', 'run', 'apps/website/index.js'],
  },

  resources: {
    cpu: '500m',
    memory: '1Gi',
  },

  autoscale: {
    minCount: 2,
    maxCount: 20,
    averageCpuUtilization: 50,
  },

  secret: {
    project: 'glitter-website',
  },

  ingress: {
    domain: {
      production: 'glitter.im',
      dev: 'glitter.pizza',
    },

    priority: {
      production: '21',
      dev: '121',
    },

    cloudfront: {
      production: {
        domainZone: 'glitter.im',
      },
    },
  },
});

new glitter.Redirect('www.website', {
  name: 'www.website',

  priority: {
    production: '22',
    dev: '122',
  },

  production: {
    from: { host: 'www.glitter.im' },
    to: { host: 'glitter.im' },
  },

  dev: {
    from: { host: 'www.glitter.pizza' },
    to: { host: 'glitter.pizza' },
  },

  code: 301,
});
