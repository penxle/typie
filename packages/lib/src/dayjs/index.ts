import 'dayjs/locale/ko';

import dayjs from 'dayjs';
import duration from 'dayjs/plugin/duration';
import isoWeek from 'dayjs/plugin/isoWeek';
import minMax from 'dayjs/plugin/minMax';
import relativeTime from 'dayjs/plugin/relativeTime';
import timezone from 'dayjs/plugin/timezone';
import utc from 'dayjs/plugin/utc';
import { formatAs } from './plugins/format-as';
import { kst } from './plugins/kst';

dayjs.extend(duration);
dayjs.extend(relativeTime);
dayjs.extend(timezone);
dayjs.extend(utc);
dayjs.extend(minMax);
dayjs.extend(isoWeek);
dayjs.extend(kst);
dayjs.extend(formatAs);

dayjs.locale('ko');
