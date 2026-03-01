import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:gap/gap.dart';
import 'package:typie/context/bottom_sheet.dart';
import 'package:typie/context/modal.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/screens/native_editor/state/controller.dart';
import 'package:typie/screens/native_editor/state/state.dart';
import 'package:typie/widgets/forms/form.dart';
import 'package:typie/widgets/forms/select.dart';
import 'package:typie/widgets/forms/text_field.dart';
import 'package:typie/widgets/horizontal_divider.dart';
import 'package:typie/widgets/tappable.dart';

const double _minContentSizeMm = 50;
const double _minPageSizeMm = 100;
const double _maxPageSizeMm = 2000;

const double _mmToPxFactor = 96 / 25.4;
const double _pxToMmFactor = 25.4 / 96;

double _mmToPx(double mm) => (mm * _mmToPxFactor).roundToDouble();
double _pxToMm(double px) => (px * _pxToMmFactor).roundToDouble();

const Map<String, Map<String, double>> _pageSizeMap = {
  'a4': {'width': 210, 'height': 297},
  'a5': {'width': 148, 'height': 210},
  'b5': {'width': 176, 'height': 250},
  'b6': {'width': 125, 'height': 176},
};

const Map<String, Map<String, double>> _defaultPageMargins = {
  'a4': {'top': 25, 'bottom': 25, 'left': 25, 'right': 25},
  'a5': {'top': 20, 'bottom': 20, 'left': 20, 'right': 20},
  'b5': {'top': 15, 'bottom': 15, 'left': 15, 'right': 15},
  'b6': {'top': 10, 'bottom': 10, 'left': 10, 'right': 10},
};

class SettingsSheet extends HookWidget {
  const SettingsSheet({required this.controller, super.key});

  final EditorController controller;

  @override
  Widget build(BuildContext context) {
    final state = useListenable(controller);
    final layoutMode = state.state.layout;
    final isPaginated = layoutMode is PaginatedLayout;
    final settings = state.state.settings;

    void handleLayoutModeChange(String mode) {
      if (mode == 'paginated') {
        final preset = _pageSizeMap['a4']!;
        final margins = _defaultPageMargins['a4']!;
        controller.dispatch({
          'type': 'setLayoutMode',
          'mode': {
            'type': 'paginated',
            'pageWidth': _mmToPx(preset['width']!),
            'pageHeight': _mmToPx(preset['height']!),
            'pageMarginTop': _mmToPx(margins['top']!),
            'pageMarginBottom': _mmToPx(margins['bottom']!),
            'pageMarginLeft': _mmToPx(margins['left']!),
            'pageMarginRight': _mmToPx(margins['right']!),
          },
        });
      } else {
        controller.dispatch({
          'type': 'setLayoutMode',
          'mode': {'type': 'continuous', 'maxWidth': 600.0},
        });
      }
    }

    void handleMaxWidthChange(int maxWidth) {
      controller.dispatch({
        'type': 'setLayoutMode',
        'mode': {'type': 'continuous', 'maxWidth': maxWidth.toDouble()},
      });
    }

    void handleParagraphIndentChange(num indent) {
      controller.dispatch({'type': 'setParagraphIndent', 'indent': indent});
    }

    void handleBlockGapChange(num gap) {
      controller.dispatch({'type': 'setBlockGap', 'gap': gap});
    }

    return AppBottomSheet(
      padding: const Pad(horizontal: 20),
      child: HookForm(
        key: ValueKey(layoutMode),
        submitMode: HookFormSubmitMode.onChange,
        onSubmit: (form) async {
          final dirtyData = form.getDirtyFieldsData();
          if (dirtyData.containsKey('layoutMode')) {
            handleLayoutModeChange(dirtyData['layoutMode'] as String);
          }
          if (dirtyData.containsKey('maxWidth')) {
            handleMaxWidthChange(dirtyData['maxWidth'] as int);
          }
          if (dirtyData.containsKey('paragraphIndent')) {
            handleParagraphIndentChange(dirtyData['paragraphIndent'] as num);
          }
          if (dirtyData.containsKey('blockGap')) {
            handleBlockGapChange(dirtyData['blockGap'] as num);
          }
        },
        builder: (context, form) {
          final int currentMaxWidth;
          if (layoutMode case final ContinuousLayout mode) {
            currentMaxWidth = mode.maxWidth.toInt();
          } else {
            currentMaxWidth = 600;
          }

          return Column(
            spacing: 16,
            children: [
              _Option(
                icon: LucideLightIcons.file_text,
                label: '레이아웃 모드',
                trailing: HookFormSelect(
                  name: 'layoutMode',
                  initialValue: isPaginated ? 'paginated' : 'continuous',
                  items: const [
                    HookFormSelectItem(label: '스크롤', value: 'continuous'),
                    HookFormSelectItem(label: '페이지', value: 'paginated'),
                  ],
                ),
              ),
              if (layoutMode case final PaginatedLayout paginatedMode) ...[
                _PageSizeSection(layoutMode: paginatedMode, dispatch: controller.dispatch),
                _PageMarginSection(layoutMode: paginatedMode, dispatch: controller.dispatch),
                HorizontalDivider(color: context.colors.borderDefault),
              ],
              if (!isPaginated)
                _Option(
                  icon: LucideLightIcons.ruler_dimension_line,
                  label: '본문 폭',
                  trailing: HookFormSelect(
                    name: 'maxWidth',
                    initialValue: currentMaxWidth,
                    items: const [
                      HookFormSelectItem(label: '400px', value: 400),
                      HookFormSelectItem(label: '600px', value: 600),
                      HookFormSelectItem(label: '800px', value: 800),
                    ],
                  ),
                ),
              _Option(
                icon: LucideLightIcons.arrow_right_to_line,
                label: '첫 줄 들여쓰기',
                trailing: HookFormSelect(
                  name: 'paragraphIndent',
                  initialValue: settings.paragraphIndent,
                  items: const [
                    HookFormSelectItem(label: '없음', value: 0),
                    HookFormSelectItem(label: '0.5칸', value: 50),
                    HookFormSelectItem(label: '1칸', value: 100),
                    HookFormSelectItem(label: '2칸', value: 200),
                  ],
                ),
              ),
              _Option(
                icon: LucideLightIcons.align_vertical_space_around,
                label: '문단 사이 간격',
                trailing: HookFormSelect(
                  name: 'blockGap',
                  initialValue: settings.blockGap,
                  items: const [
                    HookFormSelectItem(label: '없음', value: 0),
                    HookFormSelectItem(label: '0.5줄', value: 50),
                    HookFormSelectItem(label: '1줄', value: 100),
                    HookFormSelectItem(label: '2줄', value: 200),
                  ],
                ),
              ),
            ],
          );
        },
      ),
    );
  }
}

class _Option extends StatelessWidget {
  const _Option({required this.icon, required this.label, required this.trailing});

  final IconData icon;
  final String label;
  final Widget trailing;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      height: 24,
      child: Row(
        children: [
          Icon(icon, size: 20, color: context.colors.textSubtle),
          const Gap(8),
          Expanded(
            child: Text(label, style: TextStyle(fontSize: 16, color: context.colors.textSubtle)),
          ),
          trailing,
        ],
      ),
    );
  }
}

String _getPageSizePreset(PaginatedLayout layoutMode) {
  final widthMm = _pxToMm(layoutMode.pageWidth);
  final heightMm = _pxToMm(layoutMode.pageHeight);

  for (final entry in _pageSizeMap.entries) {
    if (entry.value['width'] == widthMm && entry.value['height'] == heightMm) {
      return entry.key;
    }
  }
  return 'custom';
}

double _getMaxMargin(String side, PaginatedLayout layoutMode) {
  final widthMm = _pxToMm(layoutMode.pageWidth);
  final heightMm = _pxToMm(layoutMode.pageHeight);
  final marginTopMm = _pxToMm(layoutMode.pageMarginTop);
  final marginBottomMm = _pxToMm(layoutMode.pageMarginBottom);
  final marginLeftMm = _pxToMm(layoutMode.pageMarginLeft);
  final marginRightMm = _pxToMm(layoutMode.pageMarginRight);

  return switch (side) {
    'left' => (widthMm - marginRightMm - _minContentSizeMm).clamp(0, double.infinity),
    'right' => (widthMm - marginLeftMm - _minContentSizeMm).clamp(0, double.infinity),
    'top' => (heightMm - marginBottomMm - _minContentSizeMm).clamp(0, double.infinity),
    'bottom' => (heightMm - marginTopMm - _minContentSizeMm).clamp(0, double.infinity),
    _ => 0,
  };
}

Map<String, double> _normalizeMarginsForPageSize({
  required double pageWidthMm,
  required double pageHeightMm,
  required double marginTopMm,
  required double marginBottomMm,
  required double marginLeftMm,
  required double marginRightMm,
}) {
  Map<String, double> fitAxisMargins({
    required double startMarginMm,
    required double endMarginMm,
    required double pageSizeMm,
  }) {
    final start = startMarginMm.clamp(0, double.infinity).toDouble();
    final end = endMarginMm.clamp(0, double.infinity).toDouble();
    final maxTotalMargin = (pageSizeMm - _minContentSizeMm).clamp(0, double.infinity).toDouble();
    final totalMargin = start + end;

    if (totalMargin <= maxTotalMargin) {
      return {'start': start, 'end': end};
    }

    if (totalMargin == 0) {
      return {'start': 0, 'end': 0};
    }

    final ratio = maxTotalMargin / totalMargin;
    final nextStart = start * ratio;
    return {'start': nextStart, 'end': (maxTotalMargin - nextStart).clamp(0, double.infinity).toDouble()};
  }

  final vertical = fitAxisMargins(startMarginMm: marginTopMm, endMarginMm: marginBottomMm, pageSizeMm: pageHeightMm);
  final horizontal = fitAxisMargins(startMarginMm: marginLeftMm, endMarginMm: marginRightMm, pageSizeMm: pageWidthMm);

  return {
    'top': vertical['start']!,
    'bottom': vertical['end']!,
    'left': horizontal['start']!,
    'right': horizontal['end']!,
  };
}

class _PageSizeSection extends StatelessWidget {
  const _PageSizeSection({required this.layoutMode, required this.dispatch});

  final PaginatedLayout layoutMode;
  final void Function(Map<String, dynamic>) dispatch;

  @override
  Widget build(BuildContext context) {
    void handlePagePresetChange(String preset) {
      if (preset == 'custom') {
        return;
      }

      final size = _pageSizeMap[preset]!;
      final margins = _defaultPageMargins[preset]!;
      dispatch({
        'type': 'setLayoutMode',
        'mode': {
          'type': 'paginated',
          'pageWidth': _mmToPx(size['width']!),
          'pageHeight': _mmToPx(size['height']!),
          'pageMarginTop': _mmToPx(margins['top']!),
          'pageMarginBottom': _mmToPx(margins['bottom']!),
          'pageMarginLeft': _mmToPx(margins['left']!),
          'pageMarginRight': _mmToPx(margins['right']!),
        },
      });
    }

    final widthMm = _pxToMm(layoutMode.pageWidth);
    final heightMm = _pxToMm(layoutMode.pageHeight);

    return HookForm(
      submitMode: HookFormSubmitMode.onChange,
      onSubmit: (form) async {
        final dirtyData = form.getDirtyFieldsData();
        if (dirtyData.containsKey('pageSize')) {
          handlePagePresetChange(dirtyData['pageSize'] as String);
        }
      },
      builder: (context, form) {
        return Column(
          spacing: 8,
          children: [
            _Option(
              icon: LucideLightIcons.file,
              label: '페이지 크기',
              trailing: HookFormSelect(
                name: 'pageSize',
                initialValue: _getPageSizePreset(layoutMode),
                items: const [
                  HookFormSelectItem(label: 'A4 (210×297)', value: 'a4'),
                  HookFormSelectItem(label: 'A5 (148×210)', value: 'a5'),
                  HookFormSelectItem(label: 'B5 (176×250)', value: 'b5'),
                  HookFormSelectItem(label: 'B6 (125×176)', value: 'b6'),
                  HookFormSelectItem(label: '직접 지정', value: 'custom'),
                ],
              ),
            ),
            Row(
              spacing: 8,
              children: [
                Tappable(
                  onTap: () => _editPageSize(context, 'width'),
                  child: Container(
                    padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
                    decoration: BoxDecoration(
                      border: Border.all(color: context.colors.borderStrong, width: 0.5),
                      borderRadius: BorderRadius.circular(6),
                    ),
                    child: Text(
                      '너비: ${widthMm.toStringAsFixed(0)}mm',
                      style: TextStyle(fontSize: 14, color: context.colors.textSubtle),
                    ),
                  ),
                ),
                Tappable(
                  onTap: () => _editPageSize(context, 'height'),
                  child: Container(
                    padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
                    decoration: BoxDecoration(
                      border: Border.all(color: context.colors.borderStrong, width: 0.5),
                      borderRadius: BorderRadius.circular(6),
                    ),
                    child: Text(
                      '높이: ${heightMm.toStringAsFixed(0)}mm',
                      style: TextStyle(fontSize: 14, color: context.colors.textSubtle),
                    ),
                  ),
                ),
              ],
            ),
          ],
        );
      },
    );
  }

  Future<void> _editPageSize(BuildContext context, String dimension) async {
    final currentValuePx = dimension == 'width' ? layoutMode.pageWidth : layoutMode.pageHeight;
    final currentValue = _pxToMm(currentValuePx);
    final label = dimension == 'width' ? '너비' : '높이';
    num? newValue;

    bool validatePageSize(HookFormController form, String? valueStr) {
      if (valueStr == null || valueStr.isEmpty) {
        form.setError('value', '값을 입력해주세요');
        return false;
      }

      final value = num.parse(valueStr);

      if (value < _minPageSizeMm) {
        form.setError('value', '최소 ${_minPageSizeMm.toStringAsFixed(0)} mm 이상이어야 합니다');
        return false;
      }

      if (value > _maxPageSizeMm) {
        form.setError('value', '최대 ${_maxPageSizeMm.toStringAsFixed(0)} mm 이하여야 합니다');
        return false;
      }

      form.clearError('value');
      return true;
    }

    await context.showModal(
      intercept: true,
      child: HookForm(
        onSubmit: (form) async {
          final valueStr = form.data['value'] as String?;
          if (validatePageSize(form, valueStr)) {
            newValue = num.parse(valueStr!);
          }
        },
        builder: (context, form) {
          return ConfirmModal(
            title: '페이지 $label 설정',
            confirmText: '저장',
            onConfirmValidate: () async {
              await form.submit();
              return form.errors['value'] == null && newValue != null;
            },
            onConfirm: () {},
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              spacing: 8,
              children: [
                Text('페이지 $label를 입력하세요 (mm)', style: TextStyle(fontSize: 14, color: context.colors.textSubtle)),
                HookFormTextField.collapsed(
                  initialValue: currentValue.toStringAsFixed(0),
                  name: 'value',
                  placeholder: '$label (mm)',
                  autofocus: true,
                  style: const TextStyle(fontSize: 16),
                  submitOnEnter: false,
                  keyboardType: TextInputType.number,
                  inputFormatters: [FilteringTextInputFormatter.digitsOnly],
                  onChanged: (String value) {
                    validatePageSize(form, value);
                  },
                ),
              ],
            ),
          );
        },
      ),
    );

    if (newValue != null && newValue != currentValue) {
      final nextWidthMm = dimension == 'width' ? newValue!.toDouble() : _pxToMm(layoutMode.pageWidth);
      final nextHeightMm = dimension == 'height' ? newValue!.toDouble() : _pxToMm(layoutMode.pageHeight);
      final normalizedMargins = _normalizeMarginsForPageSize(
        pageWidthMm: nextWidthMm,
        pageHeightMm: nextHeightMm,
        marginTopMm: _pxToMm(layoutMode.pageMarginTop),
        marginBottomMm: _pxToMm(layoutMode.pageMarginBottom),
        marginLeftMm: _pxToMm(layoutMode.pageMarginLeft),
        marginRightMm: _pxToMm(layoutMode.pageMarginRight),
      );
      dispatch({
        'type': 'setLayoutMode',
        'mode': {
          'type': 'paginated',
          'pageWidth': _mmToPx(nextWidthMm),
          'pageHeight': _mmToPx(nextHeightMm),
          'pageMarginTop': _mmToPx(normalizedMargins['top']!),
          'pageMarginBottom': _mmToPx(normalizedMargins['bottom']!),
          'pageMarginLeft': _mmToPx(normalizedMargins['left']!),
          'pageMarginRight': _mmToPx(normalizedMargins['right']!),
        },
      });
    }
  }
}

class _PageMarginSection extends StatelessWidget {
  const _PageMarginSection({required this.layoutMode, required this.dispatch});

  final PaginatedLayout layoutMode;
  final void Function(Map<String, dynamic>) dispatch;

  Widget _marginButton(BuildContext context, String side, String label, double valueMm) {
    return Tappable(
      onTap: () => _editPageMargin(context, side),
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
        decoration: BoxDecoration(
          border: Border.all(color: context.colors.borderStrong, width: 0.5),
          borderRadius: BorderRadius.circular(6),
        ),
        child: Text(
          '$label: ${valueMm.toStringAsFixed(0)}',
          style: TextStyle(fontSize: 14, color: context.colors.textSubtle),
        ),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    return Column(
      spacing: 8,
      children: [
        const _Option(icon: LucideLightIcons.ruler_dimension_line, label: '여백 (mm)', trailing: SizedBox.shrink()),
        Wrap(
          spacing: 8,
          runSpacing: 8,
          children: [
            _marginButton(context, 'top', '위', _pxToMm(layoutMode.pageMarginTop)),
            _marginButton(context, 'bottom', '아래', _pxToMm(layoutMode.pageMarginBottom)),
            _marginButton(context, 'left', '왼쪽', _pxToMm(layoutMode.pageMarginLeft)),
            _marginButton(context, 'right', '오른쪽', _pxToMm(layoutMode.pageMarginRight)),
          ],
        ),
      ],
    );
  }

  Future<void> _editPageMargin(BuildContext context, String side) async {
    final currentValuePx = switch (side) {
      'top' => layoutMode.pageMarginTop,
      'bottom' => layoutMode.pageMarginBottom,
      'left' => layoutMode.pageMarginLeft,
      'right' => layoutMode.pageMarginRight,
      _ => 0.0,
    };
    final currentValue = _pxToMm(currentValuePx);

    final label = switch (side) {
      'top' => '위',
      'bottom' => '아래',
      'left' => '왼쪽',
      'right' => '오른쪽',
      _ => '',
    };

    num? newValue;

    bool validateMargin(HookFormController form, String? valueStr) {
      if (valueStr == null || valueStr.isEmpty) {
        form.setError('value', '값을 입력해주세요');
        return false;
      }

      final value = num.parse(valueStr);
      final maxMargin = _getMaxMargin(side, layoutMode);

      if (value > maxMargin) {
        form.setError('value', '최대 ${maxMargin.toStringAsFixed(0)}mm까지 가능합니다');
        return false;
      }

      form.clearError('value');
      return true;
    }

    await context.showModal(
      intercept: true,
      child: HookForm(
        onSubmit: (form) async {
          final valueStr = form.data['value'] as String?;
          if (validateMargin(form, valueStr)) {
            newValue = num.parse(valueStr!);
          }
        },
        builder: (context, form) {
          final maxMargin = _getMaxMargin(side, layoutMode);
          return ConfirmModal(
            title: '$label 여백 설정',
            confirmText: '저장',
            onConfirmValidate: () async {
              await form.submit();
              return form.errors['value'] == null && newValue != null;
            },
            onConfirm: () {},
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              spacing: 8,
              children: [
                Text(
                  '$label 여백을 입력하세요 (0-${maxMargin.toStringAsFixed(0)}mm)',
                  style: TextStyle(fontSize: 14, color: context.colors.textSubtle),
                ),
                HookFormTextField.collapsed(
                  initialValue: currentValue.toStringAsFixed(0),
                  name: 'value',
                  placeholder: '여백 (mm)',
                  autofocus: true,
                  style: const TextStyle(fontSize: 16),
                  submitOnEnter: false,
                  keyboardType: TextInputType.number,
                  inputFormatters: [FilteringTextInputFormatter.digitsOnly],
                  onChanged: (String value) {
                    validateMargin(form, value);
                  },
                ),
              ],
            ),
          );
        },
      ),
    );

    if (newValue != null && newValue != currentValue) {
      dispatch({
        'type': 'setLayoutMode',
        'mode': {
          'type': 'paginated',
          'pageWidth': layoutMode.pageWidth,
          'pageHeight': layoutMode.pageHeight,
          'pageMarginTop': side == 'top' ? _mmToPx(newValue!.toDouble()) : layoutMode.pageMarginTop,
          'pageMarginBottom': side == 'bottom' ? _mmToPx(newValue!.toDouble()) : layoutMode.pageMarginBottom,
          'pageMarginLeft': side == 'left' ? _mmToPx(newValue!.toDouble()) : layoutMode.pageMarginLeft,
          'pageMarginRight': side == 'right' ? _mmToPx(newValue!.toDouble()) : layoutMode.pageMarginRight,
        },
      });
    }
  }
}
