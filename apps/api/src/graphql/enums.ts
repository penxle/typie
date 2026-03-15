import * as enums from '#/enums.ts';
import { builder } from './builder.ts';

/**
 * * Enums
 */

for (const [name, e] of Object.entries(enums)) {
  builder.enumType(e, { name });
}
