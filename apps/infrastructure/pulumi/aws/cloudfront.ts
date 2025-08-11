import * as aws from '@pulumi/aws';
import { zones } from '$aws/route53';
import { buckets } from '$aws/s3';

const createCertificate = (domain: string, ...subjectAlternativeNames: string[]) => {
  const certificate = new aws.acm.Certificate(`${domain}@cloudfront`, {
    region: 'us-east-1',
    domainName: domain,
    subjectAlternativeNames: [`*.${domain}`, ...subjectAlternativeNames],
    validationMethod: 'DNS',
  });

  new aws.acm.CertificateValidation(`${domain}@cloudfront`, {
    region: 'us-east-1',
    certificateArn: certificate.arn,
  });

  return certificate;
};

export const certificates = {
  typie_co: createCertificate('typie.co'),
  typie_dev: createCertificate('typie.dev', '*.usersite.typie.dev'),
  typie_me: createCertificate('typie.me'),
  typie_net: createCertificate('typie.net'),
  typie_io: createCertificate('typie.io'),
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
    enableAcceptEncodingBrotli: false,
    enableAcceptEncodingGzip: false,

    cookiesConfig: { cookieBehavior: 'none' },
    headersConfig: { headerBehavior: 'whitelist', headers: { items: ['Accept-Encoding'] } },
    queryStringsConfig: { queryStringBehavior: 'none' },
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

const app = new aws.cloudfront.Distribution('app', {
  enabled: true,
  aliases: ['app.typie.net'],
  httpVersion: 'http2and3',

  origins: [
    {
      originId: 'app',
      domainName: buckets.app.bucketRegionalDomainName,
      originAccessControlId: s3OriginAccessControl.id,
    },
  ],

  defaultCacheBehavior: {
    targetOriginId: 'app',
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
    acmCertificateArn: certificates.typie_net.arn,
    sslSupportMethod: 'sni-only',
    minimumProtocolVersion: 'TLSv1.2_2021',
  },

  waitForDeployment: false,
});

const cdn = new aws.cloudfront.Distribution('cdn', {
  enabled: true,
  aliases: ['cdn.typie.net'],
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
    acmCertificateArn: certificates.typie_net.arn,
    sslSupportMethod: 'sni-only',
    minimumProtocolVersion: 'TLSv1.2_2021',
  },

  waitForDeployment: false,
});

const usercontents = new aws.cloudfront.Distribution('usercontents', {
  enabled: true,
  aliases: ['typie.net'],
  httpVersion: 'http2and3',

  origins: [
    {
      originId: 'usercontents',
      domainName: buckets.usercontents.bucketRegionalDomainName,
      originAccessControlId: s3OriginAccessControl.id,
    },
    {
      originId: 'usercontents-literoom',
      // spell-checker:disable-next-line
      domainName: 'usercontents-literoo-dsqhecmpgp5romu8x8rbkcmbapn2a--ol-s3.s3.ap-northeast-2.amazonaws.com',
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

  orderedCacheBehaviors: [
    {
      targetOriginId: 'usercontents-literoom',
      pathPattern: 'images/*',
      compress: true,
      viewerProtocolPolicy: 'redirect-to-https',
      allowedMethods: ['GET', 'HEAD', 'OPTIONS'],
      cachedMethods: ['GET', 'HEAD', 'OPTIONS'],
      cachePolicyId: staticCachePolicy.id,
      originRequestPolicyId: staticOriginRequestPolicy.id,
      responseHeadersPolicyId: staticResponseHeadersPolicy.id,
    },
  ],

  restrictions: {
    geoRestriction: {
      restrictionType: 'none',
    },
  },

  viewerCertificate: {
    acmCertificateArn: certificates.typie_net.arn,
    sslSupportMethod: 'sni-only',
    minimumProtocolVersion: 'TLSv1.2_2021',
  },

  waitForDeployment: false,
});

new aws.route53.Record('app.typie.net', {
  zoneId: zones.typie_net.zoneId,
  type: 'A',
  name: 'app.typie.net',
  aliases: [
    {
      name: app.domainName,
      zoneId: app.hostedZoneId,
      evaluateTargetHealth: false,
    },
  ],
});

new aws.route53.Record('cdn.typie.net', {
  zoneId: zones.typie_net.zoneId,
  type: 'A',
  name: 'cdn.typie.net',
  aliases: [
    {
      name: cdn.domainName,
      zoneId: cdn.hostedZoneId,
      evaluateTargetHealth: false,
    },
  ],
});

new aws.route53.Record('typie.net', {
  zoneId: zones.typie_net.zoneId,
  type: 'A',
  name: 'typie.net',
  aliases: [
    {
      name: usercontents.domainName,
      zoneId: usercontents.hostedZoneId,
      evaluateTargetHealth: false,
    },
  ],
});

export const distributions = { cdn };

export const outputs = {
  AWS_CLOUDFRONT_DYNAMIC_CACHE_POLICY_ID: dynamicCachePolicy.id,
  AWS_CLOUDFRONT_DYNAMIC_ORIGIN_REQUEST_POLICY_ID: dynamicOriginRequestPolicy.id,
  AWS_CLOUDFRONT_DYNAMIC_RESPONSE_HEADERS_POLICY_ID: dynamicResponseHeadersPolicy.id,
  AWS_CLOUDFRONT_TYPIE_CO_CERTIFICATE_ARN: certificates.typie_co.arn,
  AWS_CLOUDFRONT_TYPIE_DEV_CERTIFICATE_ARN: certificates.typie_dev.arn,
  AWS_CLOUDFRONT_TYPIE_ME_CERTIFICATE_ARN: certificates.typie_me.arn,
  AWS_CLOUDFRONT_TYPIE_NET_CERTIFICATE_ARN: certificates.typie_net.arn,
  AWS_CLOUDFRONT_TYPIE_IO_CERTIFICATE_ARN: certificates.typie_io.arn,
};
