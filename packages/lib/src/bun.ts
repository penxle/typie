import { getConnInfo } from 'hono/bun';
import IPAddr from 'ipaddr.js';
import * as R from 'remeda';
import type { Context } from 'hono';

const proxies = process.env.TRUSTED_PROXIES?.split(',').map((v) => IPAddr.parseCIDR(v)) ?? [];

export const getClientAddress = (c: Context) => {
  try {
    const cf = c.req.header('CloudFront-Viewer-Address');
    if (cf) {
      const [ip] = cf.split(/:\d+$/);
      return IPAddr.process(ip).toString();
    }

    const xff = c.req.header('X-Forwarded-For');
    if (xff) {
      const ip = R.pipe(
        xff,
        R.split(','),
        R.map((v) => v.trim()),
        R.filter((v) => IPAddr.isValid(v)),
        R.map((v) => IPAddr.parse(v)),
        R.filter((v) => !proxies.some((p) => v.match(p))),
        R.findLast((v) => v.range() !== 'private'),
      );

      if (ip) {
        return ip.toString();
      }
    }

    const ip = getConnInfo(c).remote.address;
    if (ip) {
      return IPAddr.process(ip).toString();
    }
  } catch {
    // pass
  }

  return '0.0.0.0';
};
