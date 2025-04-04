import { TableCode } from '@/db';
import { isTypeOf, Site } from '../objects';

/**
 * * Types
 */

Site.implement({
  isTypeOf: isTypeOf(TableCode.SITES),
  fields: (t) => ({
    id: t.exposeID('id'),
    slug: t.exposeString('slug'),
    name: t.exposeString('name'),
  }),
});
