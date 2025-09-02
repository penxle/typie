export type SurveyData = {
  q1_1: string;
  q1_1_other: string;
  q1_2: string;
  q1_2_other: string;
  q2_1: string[];
  q2_2: string;
  q3_1: string;
  q3_1_other: string;
  q3_2: string[];
  q4_1: string[];
  q4_1_other: string;
  q4_2: string;
  q4_2_other: string;
  q5: string;
  q5_other: string;
};

export type SurveyOption = {
  value: string;
  label: string;
  hasInput?: boolean;
};

export type SurveyQuestion = {
  id: string;
  label: string;
  subtitle?: string;
  type: 'radio' | 'checkbox' | 'scale';
  options: SurveyOption[];
  maxSelect?: number;
};

export type SurveyStep = {
  id: string;
  title: string;
  subtitle: string;
  questions: SurveyQuestion[];
};
