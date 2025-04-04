import { Site } from '../objects';

/**
 * * Types
 */

Site.implement({
  fields: (t) => ({
    id: t.exposeID('id'),
    slug: t.exposeString('slug'),
    name: t.exposeString('name'),
  }),
});
