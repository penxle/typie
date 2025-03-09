import * as enums from '@/enums';
import { builder } from './builder';

/**
 * * Enums
 */

for (const [name, e] of Object.entries(enums)) {
  builder.enumType(e, { name });
}
