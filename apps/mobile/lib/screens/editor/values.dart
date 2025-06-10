import 'dart:math';

import 'package:flutter/material.dart';
import 'package:typie/styles/colors.dart';
import 'package:typie/widgets/horizontal_divider.dart';
import 'package:typie/widgets/svg_image.dart';

final editorValues = <String, List<Map<String, dynamic>>>{
  'fontFamily': [
    {'label': '프리텐다드', 'value': 'Pretendard'},
    {'label': '코펍월드돋움', 'value': 'KoPubWorldDotum'},
    {'label': '나눔바른고딕', 'value': 'NanumBarunGothic'},
    {'label': '리디바탕', 'value': 'RIDIBatang'},
    {'label': '코펍월드바탕', 'value': 'KoPubWorldBatang'},
    {'label': '나눔명조', 'value': 'NanumMyeongjo'},
  ],

  'fontSize': [
    {'label': '8pt', 'value': 8},
    {'label': '10pt', 'value': 10},
    {'label': '12pt', 'value': 12},
    {'label': '14pt', 'value': 14},
    {'label': '16pt', 'value': 16},
    {'label': '18pt', 'value': 18},
    {'label': '20pt', 'value': 20},
    {'label': '22pt', 'value': 22},
    {'label': '24pt', 'value': 24},
    {'label': '36pt', 'value': 36},
    {'label': '48pt', 'value': 48},
    {'label': '60pt', 'value': 60},
    {'label': '72pt', 'value': 72},
  ],

  'textColor': [
    {'label': '블랙', 'value': 'black', 'hex': '#09090b'},
    {'label': '그레이', 'value': 'gray', 'hex': '#71717a'},
    {'label': '화이트', 'value': 'white', 'hex': '#ffffff'},
    {'label': '레드', 'value': 'red', 'hex': '#ef4444'},
    {'label': '오렌지', 'value': 'orange', 'hex': '#f97316'},
    {'label': '앰버', 'value': 'amber', 'hex': '#f59e0b'},
    {'label': '옐로', 'value': 'yellow', 'hex': '#eab308'},
    {'label': '라임', 'value': 'lime', 'hex': '#84cc16'},
    {'label': '그린', 'value': 'green', 'hex': '#22c55e'},
    {'label': '에메랄드', 'value': 'emerald', 'hex': '#10b981'},
    {'label': '틸', 'value': 'teal', 'hex': '#14b8a6'},
    {'label': '시안', 'value': 'cyan', 'hex': '#06b6d4'},
    {'label': '스카이', 'value': 'sky', 'hex': '#0ea5e9'},
    {'label': '블루', 'value': 'blue', 'hex': '#3b82f6'},
    {'label': '인디고', 'value': 'indigo', 'hex': '#6366f1'},
    {'label': '바이올렛', 'value': 'violet', 'hex': '#8b5cf6'},
    {'label': '퍼플', 'value': 'purple', 'hex': '#a855f7'},
    {'label': '마젠타', 'value': 'fuchsia', 'hex': '#d946ef'},
    {'label': '핑크', 'value': 'pink', 'hex': '#ec4899'},
    {'label': '로즈', 'value': 'rose', 'hex': '#f43f5e'},
  ],

  'lineHeight': [
    {'label': '80%', 'value': 0.8},
    {'label': '100%', 'value': 1.0},
    {'label': '120%', 'value': 1.2},
    {'label': '140%', 'value': 1.4},
    {'label': '160%', 'value': 1.6},
    {'label': '180%', 'value': 1.8},
    {'label': '200%', 'value': 2.0},
    {'label': '220%', 'value': 2.2},
  ],

  'letterSpacing': [
    {'label': '-10%', 'value': -0.1},
    {'label': '-5%', 'value': -0.05},
    {'label': '0%', 'value': 0.0},
    {'label': '5%', 'value': 0.05},
    {'label': '10%', 'value': 0.1},
    {'label': '20%', 'value': 0.2},
    {'label': '40%', 'value': 0.4},
  ],

  'textAlign': [
    {'label': '왼쪽', 'value': 'left'},
    {'label': '중앙', 'value': 'center'},
    {'label': '오른쪽', 'value': 'right'},
    {'label': '양쪽', 'value': 'justify'},
  ],

  'blockquote': [
    {'label': '왼쪽 선', 'type': 'left-line', 'component': const VerticalDivider(color: AppColors.gray_200, thickness: 4)},
    {
      'label': '왼쪽 따옴표',
      'type': 'left-quote',
      'component': const SvgImage('icons/left-quote', height: 16, color: AppColors.gray_900),
    },
  ],

  'horizontalRule': [
    {'label': '옅은 선', 'type': 'light-line', 'component': const HorizontalDivider(color: AppColors.gray_200)},
    {
      'label': '점선',
      'type': 'dashed-line',
      'component': Row(
        mainAxisSize: MainAxisSize.min,
        spacing: 8,
        children: List.generate(6 * 2 - 1, (_) {
          return Container(width: 8, height: 1, color: AppColors.gray_700);
        }),
      ),
    },
    {
      'label': '동그라미가 있는 선',
      'type': 'circle-line',
      'component': Row(
        spacing: 10,
        children: [
          const Expanded(child: HorizontalDivider(color: AppColors.gray_700)),
          Container(
            width: 10,
            height: 10,
            decoration: const BoxDecoration(color: AppColors.gray_700, shape: BoxShape.circle),
          ),
          const Expanded(child: HorizontalDivider(color: AppColors.gray_700)),
        ],
      ),
    },
    {
      'label': '마름모가 있는 선',
      'type': 'diamond-line',
      'component': Row(
        spacing: 8,
        children: [
          const Expanded(child: HorizontalDivider(color: AppColors.gray_700)),
          Transform.rotate(
            angle: pi / 4,
            child: Container(
              width: 10,
              height: 10,
              decoration: BoxDecoration(border: Border.all(color: AppColors.gray_700)),
            ),
          ),
          const Expanded(child: HorizontalDivider(color: AppColors.gray_700)),
        ],
      ),
    },
    {
      'label': '동그라미',
      'type': 'circle',
      'component': Container(
        width: 10,
        height: 10,
        decoration: const BoxDecoration(color: AppColors.gray_700, shape: BoxShape.circle),
      ),
    },
    {
      'label': '마름모',
      'type': 'diamond',
      'component': Transform.rotate(
        angle: pi / 4,
        child: Container(
          width: 10,
          height: 10,
          decoration: BoxDecoration(border: Border.all(color: AppColors.gray_700)),
        ),
      ),
    },
    {
      'label': '세 개의 동그라미',
      'type': 'three-circles',
      'component': Row(
        mainAxisAlignment: MainAxisAlignment.center,
        spacing: 8,
        children: [
          Container(
            width: 10,
            height: 10,
            decoration: const BoxDecoration(color: AppColors.gray_700, shape: BoxShape.circle),
          ),
          Container(
            width: 10,
            height: 10,
            decoration: const BoxDecoration(color: AppColors.gray_700, shape: BoxShape.circle),
          ),
          Container(
            width: 10,
            height: 10,
            decoration: const BoxDecoration(color: AppColors.gray_700, shape: BoxShape.circle),
          ),
        ],
      ),
    },
    {
      'label': '세 개의 마름모',
      'type': 'three-diamonds',
      'component': Row(
        mainAxisAlignment: MainAxisAlignment.center,
        spacing: 8,
        children: [
          Transform.rotate(
            angle: pi / 4,
            child: Container(
              width: 10,
              height: 10,
              decoration: BoxDecoration(border: Border.all(color: AppColors.gray_700)),
            ),
          ),
          Transform.rotate(
            angle: pi / 4,
            child: Container(
              width: 10,
              height: 10,
              decoration: BoxDecoration(border: Border.all(color: AppColors.gray_700)),
            ),
          ),
          Transform.rotate(
            angle: pi / 4,
            child: Container(
              width: 10,
              height: 10,
              decoration: BoxDecoration(border: Border.all(color: AppColors.gray_700)),
            ),
          ),
        ],
      ),
    },
    {
      'label': '지그재그',
      'type': 'zigzag',
      'component': const SvgImage('icons/zigzag', height: 12, color: AppColors.gray_700),
    },
  ],

  'callout': [
    {'label': '정보', 'type': 'info'},
    {'label': '성공', 'type': 'success'},
    {'label': '경고', 'type': 'warning'},
    {'label': '주의', 'type': 'danger'},
  ],

  'paragraphIndent': [
    {'label': '없음', 'value': 0.0},
    {'label': '0.5칸', 'value': 0.5},
    {'label': '1칸', 'value': 1.0},
    {'label': '2칸', 'value': 2.0},
  ],

  'maxWidth': [
    {'label': '600px', 'value': 600},
    {'label': '800px', 'value': 800},
    {'label': '1000px', 'value': 1000},
  ],

  'blockGap': [
    {'label': '없음', 'value': 0.0},
    {'label': '0.5줄', 'value': 0.5},
    {'label': '1줄', 'value': 1.0},
    {'label': '2줄', 'value': 2.0},
  ],
};

const editorDefaultValues = <String, dynamic>{
  'fontFamily': 'Pretendard',
  'fontSize': 16,
  'textColor': 'black',
  'lineHeight': 1.6,
  'letterSpacing': 0.0,
  'textAlign': 'left',
  'blockquote': 'left-line',
  'horizontalRule': 'light-line',
  'callout': 'info',
  'paragraphIndent': 1.0,
  'maxWidth': 800,
  'blockGap': 1.0,
};
