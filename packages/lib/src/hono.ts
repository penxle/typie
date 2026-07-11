import { getConnInfo } from '@hono/node-server/conninfo';
import IPAddr from 'ipaddr.js';
import * as R from 'remeda';
import type { IncomingMessage } from 'node:http';
import type { Context } from 'hono';

const proxies = process.env.TRUSTED_PROXIES?.split(',').map((v) => IPAddr.parseCIDR(v)) ?? [];

const resolveClientAddress = (header: (name: string) => string | undefined, remoteAddress: () => string | undefined): string => {
  try {
    const cf = header('cloudfront-viewer-address');
    if (cf) {
      const [ip] = cf.split(/:\d+$/);
      return IPAddr.parse(ip).toString();
    }

    const sveltekit = header('x-client-ip');
    if (sveltekit) {
      return IPAddr.parse(sveltekit).toString();
    }

    const envoy = header('x-envoy-external-address');
    if (envoy) {
      return IPAddr.parse(envoy).toString();
    }

    const xff = header('x-forwarded-for');
    if (xff) {
      const ip = R.pipe(
        xff,
        R.split(','),
        R.map((v) => v.trim()),
        R.filter((v) => IPAddr.isValid(v)),
        R.map((v) => IPAddr.parse(v)),
        // eslint-disable-next-line unicorn/prefer-regexp-test -- v is an ipaddr.js IPv4/IPv6, .match(cidr) is CIDR matching, not String#match
        R.filter((v) => proxies.every((p) => !v.match(p))),
        R.findLast((v) => v.range() !== 'private'),
      );

      if (ip) {
        return ip.toString();
      }
    }

    const ip = remoteAddress();
    if (ip) {
      return IPAddr.parse(ip).toString();
    }
  } catch {
    // pass
  }

  return '0.0.0.0';
};

export const getClientAddress = (c: Context) =>
  resolveClientAddress(
    (name) => c.req.header(name),
    () => getConnInfo(c).remote.address,
  );

export const getClientAddressFromIncoming = (request: IncomingMessage) =>
  resolveClientAddress(
    (name) => {
      const value = request.headers[name];
      return Array.isArray(value) ? value[0] : value;
    },
    () => request.socket.remoteAddress,
  );
