import dayjs from 'dayjs';
import type { PluginFunc } from 'dayjs';

declare module 'dayjs' {
  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Dayjs {
    formatAsDate: () => string;
    formatAsDateTime: () => string;
    formatAsTime: () => string;
    formatAsSmart: () => string;
  }
}

export const formatAs: PluginFunc = (_, Dayjs) => {
  Dayjs.prototype.formatAsDate = function () {
    return this.format('YYYY. MM. DD');
  };

  Dayjs.prototype.formatAsDateTime = function () {
    return this.format('YYYY. MM. DD. HH:mm');
  };

  Dayjs.prototype.formatAsTime = function () {
    return this.format('HH:mm');
  };

  Dayjs.prototype.formatAsSmart = function () {
    const now = dayjs();

    if (this.isSame(now, 'day')) {
      return this.format('H시 mm분 ss초');
    }

    if (this.isSame(now, 'year')) {
      return this.format('M월 D일 H시 mm분 ss초');
    }

    return this.format('YYYY년 M월 D일 H시 mm분 ss초');
  };
};
