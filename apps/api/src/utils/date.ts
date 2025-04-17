import dayjs from 'dayjs';

export const getKoreanAge = (birthday: dayjs.Dayjs) => {
  return dayjs.kst().year() - birthday.year() + 1;
};
