// 순수 — env·DB·네트워크 import 없음(node:test 직접 로드)
import { effectiveResolver } from '@typie/prism';
import type { ToolPolicy } from '@typie/prism';

export const shouldPushAsk = (tool: string, policy: ToolPolicy): boolean => effectiveResolver(tool, policy) === 'user';
