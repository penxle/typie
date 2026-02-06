import 'dart:math';

import 'package:flutter/material.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/screens/native_editor/state/theme.dart';
import 'package:typie/widgets/horizontal_divider.dart';
import 'package:typie/widgets/svg_image.dart';
import 'package:typie/widgets/vertical_divider.dart';

final editorValues = <String, List<Map<String, dynamic>>>{
  'fontFamily': [
    {
      'label': '프리텐다드',
      'value': 'Pretendard',
      'weights': [100, 200, 300, 400, 500, 600, 700, 800, 900],
    },
    {
      'label': '코펍월드돋움',
      'value': 'KoPubWorldDotum',
      'weights': [500, 700],
    },
    {
      'label': '나눔바른고딕',
      'value': 'NanumBarunGothic',
      'weights': [400, 700],
    },
    {
      'label': '리디바탕',
      'value': 'RIDIBatang',
      'weights': [400],
    },
    {
      'label': '코펍월드바탕',
      'value': 'KoPubWorldBatang',
      'weights': [500, 700],
    },
    {
      'label': '나눔명조',
      'value': 'NanumMyeongjo',
      'weights': [400, 700],
    },
  ],

  'fontWeight': [
    {'label': '가장 가늘게', 'value': 100},
    {'label': '아주 가늘게', 'value': 200},
    {'label': '가늘게', 'value': 300},
    {'label': '보통', 'value': 400},
    {'label': '중간', 'value': 500},
    {'label': '약간 굵게', 'value': 600},
    {'label': '굵게', 'value': 700},
    {'label': '아주 굵게', 'value': 800},
    {'label': '가장 굵게', 'value': 900},
  ],

  'fontSize': [
    {'label': '8', 'value': 8},
    {'label': '9', 'value': 9},
    {'label': '10', 'value': 10},
    {'label': '11', 'value': 11},
    {'label': '12', 'value': 12},
    {'label': '14', 'value': 14},
    {'label': '16', 'value': 16},
    {'label': '18', 'value': 18},
    {'label': '20', 'value': 20},
    {'label': '22', 'value': 22},
    {'label': '24', 'value': 24},
    {'label': '30', 'value': 30},
    {'label': '36', 'value': 36},
    {'label': '48', 'value': 48},
    {'label': '60', 'value': 60},
    {'label': '72', 'value': 72},
    {'label': '96', 'value': 96},
  ],

  'textColor': [
    {
      'label': '블랙',
      'value': 'black',
      'color': (BuildContext context) => getEditorColor(context.theme.brightness, 'text.black'),
    },
    {
      'label': '다크 그레이',
      'value': 'darkgray',
      'color': (BuildContext context) => getEditorColor(context.theme.brightness, 'text.darkgray'),
    },
    {
      'label': '그레이',
      'value': 'gray',
      'color': (BuildContext context) => getEditorColor(context.theme.brightness, 'text.gray'),
    },
    {
      'label': '라이트 그레이',
      'value': 'lightgray',
      'color': (BuildContext context) => getEditorColor(context.theme.brightness, 'text.lightgray'),
    },
    {
      'label': '화이트',
      'value': 'white',
      'color': (BuildContext context) => getEditorColor(context.theme.brightness, 'text.white'),
    },
    {
      'label': '레드',
      'value': 'red',
      'color': (BuildContext context) => getEditorColor(context.theme.brightness, 'text.red'),
    },
    {
      'label': '오렌지',
      'value': 'orange',
      'color': (BuildContext context) => getEditorColor(context.theme.brightness, 'text.orange'),
    },
    {
      'label': '앰버',
      'value': 'amber',
      'color': (BuildContext context) => getEditorColor(context.theme.brightness, 'text.amber'),
    },
    {
      'label': '옐로',
      'value': 'yellow',
      'color': (BuildContext context) => getEditorColor(context.theme.brightness, 'text.yellow'),
    },
    {
      'label': '라임',
      'value': 'lime',
      'color': (BuildContext context) => getEditorColor(context.theme.brightness, 'text.lime'),
    },
    {
      'label': '그린',
      'value': 'green',
      'color': (BuildContext context) => getEditorColor(context.theme.brightness, 'text.green'),
    },
    {
      'label': '에메랄드',
      'value': 'emerald',
      'color': (BuildContext context) => getEditorColor(context.theme.brightness, 'text.emerald'),
    },
    {
      'label': '틸',
      'value': 'teal',
      'color': (BuildContext context) => getEditorColor(context.theme.brightness, 'text.teal'),
    },
    {
      'label': '시안',
      'value': 'cyan',
      'color': (BuildContext context) => getEditorColor(context.theme.brightness, 'text.cyan'),
    },
    {
      'label': '스카이',
      'value': 'sky',
      'color': (BuildContext context) => getEditorColor(context.theme.brightness, 'text.sky'),
    },
    {
      'label': '블루',
      'value': 'blue',
      'color': (BuildContext context) => getEditorColor(context.theme.brightness, 'text.blue'),
    },
    {
      'label': '인디고',
      'value': 'indigo',
      'color': (BuildContext context) => getEditorColor(context.theme.brightness, 'text.indigo'),
    },
    {
      'label': '바이올렛',
      'value': 'violet',
      'color': (BuildContext context) => getEditorColor(context.theme.brightness, 'text.violet'),
    },
    {
      'label': '퍼플',
      'value': 'purple',
      'color': (BuildContext context) => getEditorColor(context.theme.brightness, 'text.purple'),
    },
    {
      'label': '마젠타',
      'value': 'fuchsia',
      'color': (BuildContext context) => getEditorColor(context.theme.brightness, 'text.fuchsia'),
    },
    {
      'label': '핑크',
      'value': 'pink',
      'color': (BuildContext context) => getEditorColor(context.theme.brightness, 'text.pink'),
    },
    {
      'label': '로즈',
      'value': 'rose',
      'color': (BuildContext context) => getEditorColor(context.theme.brightness, 'text.rose'),
    },
  ],

  'textBackgroundColor': [
    {'label': '배경 없음', 'value': 'none', 'color': null},
    {
      'label': '그레이',
      'value': 'gray',
      'color': (BuildContext context) => getEditorColor(context.theme.brightness, 'bg.gray'),
    },
    {
      'label': '레드',
      'value': 'red',
      'color': (BuildContext context) => getEditorColor(context.theme.brightness, 'bg.red'),
    },
    {
      'label': '오렌지',
      'value': 'orange',
      'color': (BuildContext context) => getEditorColor(context.theme.brightness, 'bg.orange'),
    },
    {
      'label': '옐로',
      'value': 'yellow',
      'color': (BuildContext context) => getEditorColor(context.theme.brightness, 'bg.yellow'),
    },
    {
      'label': '그린',
      'value': 'green',
      'color': (BuildContext context) => getEditorColor(context.theme.brightness, 'bg.green'),
    },
    {
      'label': '블루',
      'value': 'blue',
      'color': (BuildContext context) => getEditorColor(context.theme.brightness, 'bg.blue'),
    },
    {
      'label': '퍼플',
      'value': 'purple',
      'color': (BuildContext context) => getEditorColor(context.theme.brightness, 'bg.purple'),
    },
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
    {
      'label': '왼쪽 선',
      'type': 'left_line',
      'widget': Builder(
        builder: (context) => AppVerticalDivider(color: context.colors.borderDefault, width: 4, height: 24),
      ),
    },
    {
      'label': '왼쪽 따옴표',
      'type': 'left_quote',
      'widget': Builder(
        builder: (context) => SvgImage('icons/left-quote', height: 16, color: context.colors.textDefault),
      ),
    },
    {'label': '보낸 메시지', 'type': 'message_sent'},
    {'label': '받은 메시지', 'type': 'message_received'},
  ],

  'horizontalRule': [
    {
      'label': '선',
      'type': 'line',
      'widget': Builder(builder: (context) => HorizontalDivider(color: context.colors.textSubtle)),
    },
    {
      'label': '점선',
      'type': 'dashed_line',
      'widget': LayoutBuilder(
        builder: (context, constraints) {
          const dashWidth = 8.0;
          const gapWidth = 8.0;
          final availableWidth = constraints.maxWidth;
          final dashCount = (availableWidth / (dashWidth + gapWidth)).floor();

          return Row(
            mainAxisAlignment: MainAxisAlignment.center,
            children: List.generate(dashCount, (index) {
              return Container(
                width: dashWidth,
                height: 1,
                color: context.colors.textSubtle,
                margin: EdgeInsets.only(right: index < dashCount - 1 ? gapWidth : 0),
              );
            }),
          );
        },
      ),
    },
    {
      'label': '동그라미가 있는 선',
      'type': 'circle_line',
      'widget': Builder(
        builder: (context) => Row(
          spacing: 10,
          children: [
            Expanded(child: HorizontalDivider(color: context.colors.textSubtle)),
            Container(
              width: 10,
              height: 10,
              decoration: BoxDecoration(color: context.colors.textSubtle, shape: BoxShape.circle),
            ),
            Expanded(child: HorizontalDivider(color: context.colors.textSubtle)),
          ],
        ),
      ),
    },
    {
      'label': '마름모가 있는 선',
      'type': 'diamond_line',
      'widget': Builder(
        builder: (context) => Row(
          spacing: 8,
          children: [
            Expanded(child: HorizontalDivider(color: context.colors.textSubtle)),
            Transform.rotate(
              angle: pi / 4,
              child: Container(
                width: 10,
                height: 10,
                decoration: BoxDecoration(border: Border.all(color: context.colors.textSubtle)),
              ),
            ),
            Expanded(child: HorizontalDivider(color: context.colors.textSubtle)),
          ],
        ),
      ),
    },
    {
      'label': '동그라미',
      'type': 'circle',
      'widget': Builder(
        builder: (context) => Container(
          width: 10,
          height: 10,
          decoration: BoxDecoration(color: context.colors.textSubtle, shape: BoxShape.circle),
        ),
      ),
    },
    {
      'label': '마름모',
      'type': 'diamond',
      'widget': Builder(
        builder: (context) => Transform.rotate(
          angle: pi / 4,
          child: Container(
            width: 10,
            height: 10,
            decoration: BoxDecoration(border: Border.all(color: context.colors.textSubtle)),
          ),
        ),
      ),
    },
    {
      'label': '세 개의 동그라미',
      'type': 'three_circles',
      'widget': Builder(
        builder: (context) => Row(
          mainAxisAlignment: MainAxisAlignment.center,
          spacing: 8,
          children: [
            Container(
              width: 10,
              height: 10,
              decoration: BoxDecoration(color: context.colors.textSubtle, shape: BoxShape.circle),
            ),
            Container(
              width: 10,
              height: 10,
              decoration: BoxDecoration(color: context.colors.textSubtle, shape: BoxShape.circle),
            ),
            Container(
              width: 10,
              height: 10,
              decoration: BoxDecoration(color: context.colors.textSubtle, shape: BoxShape.circle),
            ),
          ],
        ),
      ),
    },
    {
      'label': '세 개의 마름모',
      'type': 'three_diamonds',
      'widget': Builder(
        builder: (context) => Row(
          mainAxisAlignment: MainAxisAlignment.center,
          spacing: 8,
          children: [
            Transform.rotate(
              angle: pi / 4,
              child: Container(
                width: 10,
                height: 10,
                decoration: BoxDecoration(border: Border.all(color: context.colors.textSubtle)),
              ),
            ),
            Transform.rotate(
              angle: pi / 4,
              child: Container(
                width: 10,
                height: 10,
                decoration: BoxDecoration(border: Border.all(color: context.colors.textSubtle)),
              ),
            ),
            Transform.rotate(
              angle: pi / 4,
              child: Container(
                width: 10,
                height: 10,
                decoration: BoxDecoration(border: Border.all(color: context.colors.textSubtle)),
              ),
            ),
          ],
        ),
      ),
    },
    {
      'label': '지그재그',
      'type': 'zigzag',
      'widget': Builder(builder: (context) => SvgImage('icons/zigzag', height: 12, color: context.colors.textSubtle)),
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
    {'label': '400px', 'value': 400},
    {'label': '600px', 'value': 600},
    {'label': '800px', 'value': 800},
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
  'fontWeight': 400,
  'fontSize': 12,
  'textColor': 'black',
  'textBackgroundColor': 'none',
  'lineHeight': 1.6,
  'letterSpacing': 0.0,
  'textAlign': 'left',
  'blockquote': 'left_line',
  'horizontalRule': 'line',
  'callout': 'info',
  'paragraphIndent': 1.0,
  'maxWidth': 800,
  'blockGap': 1.0,
};
