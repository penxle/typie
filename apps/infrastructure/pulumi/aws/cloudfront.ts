import * as aws from '@pulumi/aws';
import { usEast1 } from '$aws/providers';
import { zones } from '$aws/route53';
import { buckets } from '$aws/s3';

const createCertificate = (domain: string) => {
  const certificate = new aws.acm.Certificate(
    `${domain}@cloudfront`,
    {
      domainName: domain,
      subjectAlternativeNames: [`*.${domain}`],
      validationMethod: 'DNS',
    },
    { provider: usEast1 },
  );

  new aws.acm.CertificateValidation(
    `${domain}@cloudfront`,
    {
      certificateArn: certificate.arn,
    },
    { provider: usEast1 },
  );

  return certificate;
};

export const certificates = {
  // glitter_im: createCertificate('glitter.im'),
  glitter_pizza: createCertificate('glitter.pizza'),
  glttr_io: createCertificate('glttr.io'),
};

const s3OriginAccessControl = new aws.cloudfront.OriginAccessControl('s3', {
  name: 's3',
  description: 'Origin access control for S3 origins',

  originAccessControlOriginType: 's3',
  signingBehavior: 'always',
  signingProtocol: 'sigv4',
});

const dynamicCachePolicy = new aws.cloudfront.CachePolicy('dynamic', {
  name: 'DynamicContents',
  comment: 'Cache policy for dynamic contents',

  minTtl: 0,
  defaultTtl: 0,
  maxTtl: 31_536_000,

  parametersInCacheKeyAndForwardedToOrigin: {
    enableAcceptEncodingBrotli: true,
    enableAcceptEncodingGzip: true,

    cookiesConfig: { cookieBehavior: 'none' },
    queryStringsConfig: { queryStringBehavior: 'none' },
    headersConfig: { headerBehavior: 'none' },
  },
});

const dynamicOriginRequestPolicy = new aws.cloudfront.OriginRequestPolicy('dynamic', {
  name: 'DynamicContents',
  comment: 'Origin request policy for dynamic contents',

  cookiesConfig: { cookieBehavior: 'all' },
  headersConfig: {
    headerBehavior: 'allViewerAndWhitelistCloudFront',
    headers: {
      items: [
        'CloudFront-Viewer-Address',
        'CloudFront-Viewer-Country-Name',
        'CloudFront-Viewer-Country-Region-Name',
        'CloudFront-Viewer-City',
      ],
    },
  },
  queryStringsConfig: { queryStringBehavior: 'all' },
});

const dynamicResponseHeadersPolicy = new aws.cloudfront.ResponseHeadersPolicy('dynamic', {
  name: 'DynamicContents',
  comment: 'Response headers policy for dynamic contents',

  securityHeadersConfig: {
    strictTransportSecurity: {
      override: true,
      accessControlMaxAgeSec: 31_536_000,
      includeSubdomains: true,
      preload: true,
    },
  },
});

const staticCachePolicy = new aws.cloudfront.CachePolicy('static', {
  name: 'StaticOrigin',
  comment: 'Cache policy for static contents',

  minTtl: 0,
  defaultTtl: 86_400,
  maxTtl: 31_536_000,

  parametersInCacheKeyAndForwardedToOrigin: {
    enableAcceptEncodingBrotli: true,
    enableAcceptEncodingGzip: true,

    cookiesConfig: { cookieBehavior: 'none' },
    headersConfig: { headerBehavior: 'none' },
    queryStringsConfig: { queryStringBehavior: 'all' },
  },
});

const staticOriginRequestPolicy = new aws.cloudfront.OriginRequestPolicy('static', {
  name: 'StaticOrigin',
  comment: 'Origin request policy for static origins',

  cookiesConfig: { cookieBehavior: 'none' },
  headersConfig: { headerBehavior: 'none' },
  queryStringsConfig: { queryStringBehavior: 'all' },
});

const staticResponseHeadersPolicy = new aws.cloudfront.ResponseHeadersPolicy('static', {
  name: 'StaticOrigin',
  comment: 'Response headers policy for static origins',

  corsConfig: {
    accessControlAllowOrigins: { items: ['*'] },
    accessControlAllowHeaders: { items: ['*'] },
    accessControlAllowMethods: { items: ['GET'] },
    accessControlAllowCredentials: false,
    originOverride: true,
  },

  customHeadersConfig: {
    items: [
      {
        header: 'Cache-Control',
        value: 'public, max-age=31536000, immutable',
        override: true,
      },
    ],
  },

  securityHeadersConfig: {
    strictTransportSecurity: {
      override: true,
      accessControlMaxAgeSec: 31_536_000,
      includeSubdomains: true,
      preload: true,
    },
  },
});

const cdn = new aws.cloudfront.Distribution('cdn', {
  enabled: true,
  aliases: ['cdn.glttr.io'],
  httpVersion: 'http2and3',

  origins: [
    {
      originId: 'cdn',
      domainName: buckets.cdn.bucketRegionalDomainName,
      originAccessControlId: s3OriginAccessControl.id,
    },
  ],

  defaultCacheBehavior: {
    targetOriginId: 'cdn',
    compress: true,
    viewerProtocolPolicy: 'redirect-to-https',
    allowedMethods: ['GET', 'HEAD', 'OPTIONS'],
    cachedMethods: ['GET', 'HEAD', 'OPTIONS'],
    cachePolicyId: staticCachePolicy.id,
    originRequestPolicyId: staticOriginRequestPolicy.id,
    responseHeadersPolicyId: staticResponseHeadersPolicy.id,
  },

  restrictions: {
    geoRestriction: {
      restrictionType: 'none',
    },
  },

  viewerCertificate: {
    acmCertificateArn: certificates.glttr_io.arn,
    sslSupportMethod: 'sni-only',
    minimumProtocolVersion: 'TLSv1.2_2021',
  },

  waitForDeployment: false,
});

new aws.route53.Record('cdn.glttr.io', {
  zoneId: zones.glttr_io.zoneId,
  type: 'A',
  name: 'cdn.glttr.io',
  aliases: [
    {
      name: cdn.domainName,
      zoneId: cdn.hostedZoneId,
      evaluateTargetHealth: false,
    },
  ],
});

const usercontents = new aws.cloudfront.Distribution('usercontents', {
  enabled: true,
  aliases: ['usercontents.glttr.io'],
  httpVersion: 'http2and3',

  origins: [
    {
      originId: 'usercontents',
      domainName: buckets.usercontents.bucketRegionalDomainName,
      originAccessControlId: s3OriginAccessControl.id,
    },
  ],

  defaultCacheBehavior: {
    targetOriginId: 'usercontents',
    compress: true,
    viewerProtocolPolicy: 'redirect-to-https',
    allowedMethods: ['GET', 'HEAD', 'OPTIONS'],
    cachedMethods: ['GET', 'HEAD', 'OPTIONS'],
    cachePolicyId: staticCachePolicy.id,
    originRequestPolicyId: staticOriginRequestPolicy.id,
    responseHeadersPolicyId: staticResponseHeadersPolicy.id,
  },

  restrictions: {
    geoRestriction: {
      restrictionType: 'none',
    },
  },

  viewerCertificate: {
    acmCertificateArn: certificates.glttr_io.arn,
    sslSupportMethod: 'sni-only',
    minimumProtocolVersion: 'TLSv1.2_2021',
  },

  waitForDeployment: false,
});

new aws.route53.Record('usercontents.glttr.io', {
  zoneId: zones.glttr_io.zoneId,
  type: 'A',
  name: 'usercontents.glttr.io',
  aliases: [
    {
      name: usercontents.domainName,
      zoneId: usercontents.hostedZoneId,
      evaluateTargetHealth: false,
    },
  ],
});

export const distributions = { cdn, usercontents };

export const outputs = {
  AWS_CLOUDFRONT_DYNAMIC_CACHE_POLICY_ID: dynamicCachePolicy.id,
  AWS_CLOUDFRONT_DYNAMIC_ORIGIN_REQUEST_POLICY_ID: dynamicOriginRequestPolicy.id,
  AWS_CLOUDFRONT_DYNAMIC_RESPONSE_HEADERS_POLICY_ID: dynamicResponseHeadersPolicy.id,
};
