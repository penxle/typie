import DOMPurify from 'isomorphic-dompurify';
import { meili } from '@/search';
import { assertSitePermission } from '@/utils/permission';
import { builder } from '../builder';
import { Post } from '../objects';

type PostSearchData = {
  id: string;
  title?: string;
  subtitle?: string;
  text: string;
};

type SearchResult = {
  id: string;
  _formatted?: Partial<PostSearchData>;
};

const sanitizeHtmlOnlyEm = (dirty: string | undefined) => {
  return dirty
    ? DOMPurify.sanitize(dirty, {
        ALLOWED_TAGS: ['em'],
      })
    : undefined;
};

const PostSearchHighlight = builder.objectRef<Partial<PostSearchData>>('PostSearchHighlight');
PostSearchHighlight.implement({
  fields: (t) => ({
    title: t.string({ nullable: true, resolve: (highlight) => sanitizeHtmlOnlyEm(highlight.title) }),
    subtitle: t.string({ nullable: true, resolve: (highlight) => sanitizeHtmlOnlyEm(highlight.subtitle) }),
    text: t.string({ nullable: true, resolve: (highlight) => sanitizeHtmlOnlyEm(highlight.text) }),
  }),
});

const IPostSearchHit = builder.interfaceRef<SearchResult>('IPostSearchHit');
IPostSearchHit.implement({
  fields: (t) => ({
    highlight: t.expose('_formatted', { type: PostSearchHighlight, nullable: true }),
  }),
});

const PostSearchHit = builder.objectRef<SearchResult>('PostSearchHit');
PostSearchHit.implement({
  interfaces: [IPostSearchHit],
  fields: (t) => ({
    post: t.field({ type: Post, resolve: (post) => post.id }),
  }),
});

type CountableSearchResult<T> = {
  estimatedTotalHits: number;
  hits: T[];
};

const SearchPostResult = builder.objectRef<CountableSearchResult<SearchResult>>('SearchPostResult');
SearchPostResult.implement({
  fields: (t) => ({
    estimatedTotalHits: t.exposeInt('estimatedTotalHits'),
    hits: t.field({ type: [PostSearchHit], resolve: (searchResult) => searchResult.hits }),
  }),
});

builder.queryFields((t) => ({
  searchPosts: t.withAuth({ session: true }).fieldWithInput({
    type: SearchPostResult,
    input: {
      // Meilisearch 필터 인젝션 방지용
      siteId: t.input.id({ validate: { regex: /^[0-9A-Z]+$/ } }),
      query: t.input.string(),
    },
    resolve: async (_, { input }, ctx) => {
      await assertSitePermission({
        userId: ctx.session.userId,
        siteId: input.siteId,
        ctx,
      });

      return await meili.index('posts').search<SearchResult>(input.query, {
        filter: [`siteId = ${input.siteId}`],
        attributesToCrop: ['*'],
        attributesToHighlight: ['title', 'subtitle', 'text'],
      });
    },
  }),
}));
