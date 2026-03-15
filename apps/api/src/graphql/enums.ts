import * as enums from '@typie/lib/enums';
import { builder } from './builder.ts';

/**
 * * Enums
 */

for (const [name, e] of Object.entries(enums)) {
  builder.enumType(e, { name });
}
