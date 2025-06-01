import 'dayjs/locale/ko.js';

import dayjs from 'dayjs';
import duration from 'dayjs/plugin/duration.js';
import isoWeek from 'dayjs/plugin/isoWeek.js';
import minMax from 'dayjs/plugin/minMax.js';
import relativeTime from 'dayjs/plugin/relativeTime.js';
import timezone from 'dayjs/plugin/timezone.js';
import utc from 'dayjs/plugin/utc.js';
import { formatAs } from './plugins/format-as.ts';
import { kst } from './plugins/kst.ts';

dayjs.extend(duration);
dayjs.extend(relativeTime);
dayjs.extend(timezone);
dayjs.extend(utc);
dayjs.extend(minMax);
dayjs.extend(isoWeek);
dayjs.extend(kst);
dayjs.extend(formatAs);

dayjs.locale('ko');
