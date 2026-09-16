import ky from 'ky';
import { env } from '#/env.ts';

type IframelyLink = { href?: string; media?: { width?: number } };

type IframelyResponse = {
  error?: string;
  html?: string;
  meta?: { title?: string; description?: string; medium?: string };
  links?: { thumbnail?: IframelyLink[]; icon?: IframelyLink[] };
};

// Iframely serves multiple variants per rel; take the widest one we know a size for.
const pickWidest = (links: IframelyLink[] | undefined) => {
  const candidates = (links ?? []).filter((link) => !!link.href);
  if (candidates.length === 0) {
    return null;
  }

  return candidates.reduce((best, link) => ((link.media?.width ?? 0) > (best.media?.width ?? 0) ? link : best)).href ?? null;
};

export const unfurl = async (url: string) => {
  const resp = await ky('https://iframe.ly/api/iframely', {
    searchParams: {
      api_key: env.IFRAMELY_API_KEY,
      url,
      omit_script: 1,
    },
  }).then((res) => res.json<IframelyResponse>());

  if (resp.error) {
    throw new Error(resp.error);
  }

  return {
    // oEmbed exposed this as `type`; the Iframely endpoint calls the same thing `meta.medium`.
    type: resp.meta?.medium ?? 'link',
    title: resp.meta?.title,
    description: resp.meta?.description,
    thumbnailUrl: pickWidest(resp.links?.thumbnail),
    faviconUrl: pickWidest(resp.links?.icon),
    html: resp.html,
  };
};
