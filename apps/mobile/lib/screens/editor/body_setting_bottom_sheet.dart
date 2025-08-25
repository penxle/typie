import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:gap/gap.dart';
import 'package:mixpanel_flutter/mixpanel_flutter.dart';
import 'package:typie/context/bottom_sheet.dart';
import 'package:typie/context/modal.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/icons/lucide_lab.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/screens/editor/page_layout.dart' as page_utils;
import 'package:typie/screens/editor/schema.dart';
import 'package:typie/screens/editor/scope.dart';
import 'package:typie/widgets/forms/form.dart';
import 'package:typie/widgets/forms/select.dart';
import 'package:typie/widgets/forms/text_field.dart';
import 'package:typie/widgets/horizontal_divider.dart';
import 'package:typie/widgets/tappable.dart';

class BodySettingBottomSheet extends HookWidget {
  const BodySettingBottomSheet({super.key, required this.scope});

  final EditorStateScope scope;

  @override
  Widget build(BuildContext context) {
    final mixpanel = useService<Mixpanel>();
    final yjsState = useValueListenable(scope.yjsState);
    final proseMirrorState = useValueListenable(scope.proseMirrorState);
    final incompatibleBlocks = useState<List<String>>([]);

    final pageLayout = yjsState?.pageLayout;

    String getBlockName(String blockType) {
      switch (blockType) {
        case 'blockquote':
          return '인용구';
        case 'callout':
          return '강조';
        case 'fold':
          return '접기';
        case 'table':
          return '표';
        case 'code_block':
          return '코드';
        case 'html_block':
          return 'HTML';
        default:
          return blockType;
      }
    }

    IconData getBlockIcon(String blockName) {
      switch (blockName) {
        case '인용구':
          return LucideLightIcons.quote;
        case '강조':
          return LucideLightIcons.message_square_warning;
        case '접기':
          return LucideLightIcons.chevrons_down_up;
        case '표':
          return LucideLightIcons.table;
        case '코드':
          return LucideLightIcons.code;
        case 'HTML':
          return LucideLightIcons.code_xml;
        default:
          return LucideLightIcons.file_text;
      }
    }

    Future<List<String>> getIncompatibleBlocks() async {
      try {
        final result = await scope.webViewController.value?.callProcedure('getIncompatibleBlocks');
        if (result != null && result is List) {
          return result.cast<String>();
        }
      } catch (_) {}
      return [];
    }

    Future<void> handleLayoutModeChange(String mode, VoidCallback onCancel) async {
      if (mode == 'page') {
        final blocks = await getIncompatibleBlocks();
        incompatibleBlocks.value = blocks;

        if (blocks.isNotEmpty && context.mounted) {
          await context.showModal(
            child: ConfirmModal(
              title: '페이지 모드 전환',
              onConfirm: () async {
                await scope.webViewController.value?.emitEvent('setLayoutMode', {
                  'mode': mode,
                  'convertIncompatibleBlocks': true,
                });
                await mixpanel.track('toggle_post_page_layout', properties: {'enabled': mode});
              },
              onCancel: () {
                onCancel();
              },
              confirmText: '해제 및 전환',
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                spacing: 16,
                children: [
                  Text(
                    '페이지 모드에서는 일부 블록을 지원하지 않아요. 다음 블록들을 해제해서 일반 문단으로 변환할까요?',
                    style: TextStyle(fontSize: 15, color: context.colors.textSubtle),
                  ),
                  Container(
                    padding: const EdgeInsets.all(12),
                    decoration: BoxDecoration(
                      color: context.colors.surfaceSubtle,
                      borderRadius: BorderRadius.circular(6),
                    ),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      spacing: 8,
                      children: blocks
                          .map(getBlockName)
                          .map(
                            (block) => Row(
                              children: [
                                Icon(getBlockIcon(block), size: 16, color: context.colors.textDefault),
                                const Gap(8),
                                Text(
                                  block,
                                  style: TextStyle(
                                    fontSize: 14,
                                    fontWeight: FontWeight.w500,
                                    color: context.colors.textDefault,
                                  ),
                                ),
                              ],
                            ),
                          )
                          .toList(),
                    ),
                  ),
                ],
              ),
            ),
          );
        } else {
          await scope.webViewController.value?.emitEvent('setLayoutMode', {'mode': mode});
          await mixpanel.track('toggle_post_page_layout', properties: {'enabled': mode});
        }
      } else {
        await scope.webViewController.value?.emitEvent('setLayoutMode', {'mode': mode});
        await mixpanel.track('toggle_post_page_layout', properties: {'enabled': mode});
      }
    }

    return AppBottomSheet(
      padding: const Pad(horizontal: 20),
      child: HookForm(
        submitMode: HookFormSubmitMode.onChange,
        onSubmit: (form) async {
          final dirtyData = form.getDirtyFieldsData();
          if (dirtyData.containsKey('layoutMode')) {
            await handleLayoutModeChange(dirtyData['layoutMode'] as String, () {
              form.setValue('layoutMode', yjsState?.layoutMode ?? 'scroll');
            });
          }
          if (dirtyData.containsKey('pageSize')) {
            final preset = dirtyData['pageSize'] as String;
            if (preset != 'custom') {
              final newLayout = page_utils.createDefaultPageLayout(preset);
              await scope.webViewController.value?.emitEvent('setPageLayout', newLayout.toJson());
            }
          }
          if (dirtyData.containsKey('maxWidth')) {
            await scope.command('max_width', attrs: {'maxWidth': dirtyData['maxWidth']});
          }
          if (dirtyData.containsKey('paragraphIndent')) {
            await scope.command('body', attrs: {'paragraphIndent': dirtyData['paragraphIndent']});
          }
          if (dirtyData.containsKey('blockGap')) {
            await scope.command('body', attrs: {'blockGap': dirtyData['blockGap']});
          }
        },
        builder: (context, form) {
          final isPageLayoutEnabled = yjsState?.layoutMode == 'page';
          return Column(
            spacing: 16,
            children: [
              _Option(
                icon: LucideLabIcons.text_square,
                label: '레이아웃 모드',
                trailing: HookFormSelect(
                  name: 'layoutMode',
                  initialValue: yjsState?.layoutMode ?? 'scroll',
                  items: const [
                    HookFormSelectItem(label: '스크롤', value: 'scroll'),
                    HookFormSelectItem(label: '페이지', value: 'page'),
                  ],
                ),
              ),
              if (isPageLayoutEnabled && pageLayout != null) ...[
                _PageSizeSection(pageLayout: pageLayout, scope: scope),
                _PageMarginSection(scope: scope, pageLayout: pageLayout),
                HorizontalDivider(color: context.colors.borderDefault),
              ],
              if (!isPageLayoutEnabled)
                _Option(
                  icon: LucideLightIcons.ruler_dimension_line,
                  label: '본문 폭',
                  trailing: HookFormSelect(
                    name: 'maxWidth',
                    initialValue: yjsState?.maxWidth ?? 800,
                    items: const [
                      HookFormSelectItem(label: '600px', value: 600),
                      HookFormSelectItem(label: '800px', value: 800),
                      HookFormSelectItem(label: '1000px', value: 1000),
                    ],
                  ),
                ),
              _Option(
                icon: LucideLightIcons.arrow_right_to_line,
                label: '첫 줄 들여쓰기',
                trailing: HookFormSelect(
                  name: 'paragraphIndent',
                  initialValue: (proseMirrorState?.nodes.isNotEmpty ?? false)
                      ? (proseMirrorState!.nodes.first.attrs?['paragraphIndent'] ?? 1)
                      : 1,
                  items: const [
                    HookFormSelectItem(label: '없음', value: 0),
                    HookFormSelectItem(label: '0.5칸', value: 0.5),
                    HookFormSelectItem(label: '1칸', value: 1),
                    HookFormSelectItem(label: '2칸', value: 2),
                  ],
                ),
              ),
              _Option(
                icon: LucideLightIcons.align_vertical_space_around,
                label: '문단 사이 간격',
                trailing: HookFormSelect(
                  name: 'blockGap',
                  initialValue: (proseMirrorState?.nodes.isNotEmpty ?? false)
                      ? (proseMirrorState!.nodes.first.attrs?['blockGap'] ?? 1)
                      : 1,
                  items: const [
                    HookFormSelectItem(label: '없음', value: 0),
                    HookFormSelectItem(label: '0.5줄', value: 0.5),
                    HookFormSelectItem(label: '1줄', value: 1),
                    HookFormSelectItem(label: '2줄', value: 2),
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

Future<void> _editPageMargin(BuildContext context, EditorStateScope scope, PageLayout layout, String side) async {
  final currentValue = switch (side) {
    'top' => layout.marginTop,
    'bottom' => layout.marginBottom,
    'left' => layout.marginLeft,
    'right' => layout.marginRight,
    _ => 0.0,
  };

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
    final maxMargin = page_utils.getMaxMargin(side, layout);

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
        final maxMargin = page_utils.getMaxMargin(side, layout);
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
    final updateData = {
      switch (side) {
        'top' => 'marginTop',
        'bottom' => 'marginBottom',
        'left' => 'marginLeft',
        'right' => 'marginRight',
        _ => '',
      }: newValue!
          .toDouble(),
    };
    await scope.webViewController.value?.emitEvent('setPageLayout', updateData);
  }
}

Future<void> _editPageSize(BuildContext context, EditorStateScope scope, PageLayout layout, String dimension) async {
  final currentValue = dimension == 'width' ? layout.width : layout.height;
  final label = dimension == 'width' ? '너비' : '높이';
  num? newValue;

  bool validatePageSize(HookFormController form, String? valueStr) {
    if (valueStr == null || valueStr.isEmpty) {
      form.setError('value', '값을 입력해주세요');
      return false;
    }

    final value = num.parse(valueStr);

    if (value < page_utils.minPageSizeMm) {
      form.setError('value', '최소 ${page_utils.minPageSizeMm} mm 이상이어야 합니다');
      return false;
    }

    if (value > page_utils.maxPageSizeMm) {
      form.setError('value', '최대 ${page_utils.maxPageSizeMm} mm 이하여야 합니다');
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
    final updateData = <String, double>{};

    if (dimension == 'width') {
      updateData['width'] = newValue!.toDouble();
      final tempLayout = layout.copyWith(width: newValue!.toDouble());
      final maxLeft = page_utils.getMaxMargin('left', tempLayout);
      final maxRight = page_utils.getMaxMargin('right', tempLayout);

      if (layout.marginLeft > maxLeft) {
        updateData['marginLeft'] = maxLeft;
      }
      if (layout.marginRight > maxRight) {
        updateData['marginRight'] = maxRight;
      }
    } else {
      updateData['height'] = newValue!.toDouble();
      final tempLayout = layout.copyWith(height: newValue!.toDouble());
      final maxTop = page_utils.getMaxMargin('top', tempLayout);
      final maxBottom = page_utils.getMaxMargin('bottom', tempLayout);

      if (layout.marginTop > maxTop) {
        updateData['marginTop'] = maxTop;
      }
      if (layout.marginBottom > maxBottom) {
        updateData['marginBottom'] = maxBottom;
      }
    }

    await scope.webViewController.value?.emitEvent('setPageLayout', updateData);
  }
}

String _getPageSizePreset(PageLayout layout) {
  for (final entry in page_utils.pageSizeMap.entries) {
    if (layout.width == entry.value['width'] && layout.height == entry.value['height']) {
      return entry.key;
    }
  }
  return 'custom';
}

class _PageSizeSection extends HookWidget {
  const _PageSizeSection({required this.pageLayout, required this.scope});

  final PageLayout pageLayout;
  final EditorStateScope scope;

  @override
  Widget build(BuildContext context) {
    return Column(
      spacing: 8,
      children: [
        _Option(
          icon: LucideLightIcons.file,
          label: '페이지 크기',
          trailing: HookFormSelect(
            name: 'pageSize',
            initialValue: _getPageSizePreset(pageLayout),
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
              onTap: () => _editPageSize(context, scope, pageLayout, 'width'),
              child: Container(
                padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
                decoration: BoxDecoration(
                  border: Border.all(color: context.colors.borderStrong, width: 0.5),
                  borderRadius: BorderRadius.circular(6),
                ),
                child: Text(
                  '너비: ${pageLayout.width.toStringAsFixed(0)}mm',
                  style: TextStyle(fontSize: 14, color: context.colors.textSubtle),
                ),
              ),
            ),
            Tappable(
              onTap: () => _editPageSize(context, scope, pageLayout, 'height'),
              child: Container(
                padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
                decoration: BoxDecoration(
                  border: Border.all(color: context.colors.borderStrong, width: 0.5),
                  borderRadius: BorderRadius.circular(6),
                ),
                child: Text(
                  '높이: ${pageLayout.height.toStringAsFixed(0)}mm',
                  style: TextStyle(fontSize: 14, color: context.colors.textSubtle),
                ),
              ),
            ),
          ],
        ),
      ],
    );
  }
}

class _PageMarginSection extends StatelessWidget {
  const _PageMarginSection({required this.scope, required this.pageLayout});

  final EditorStateScope scope;
  final PageLayout pageLayout;

  @override
  Widget build(BuildContext context) {
    return Column(
      spacing: 8,
      children: [
        const _Option(icon: LucideLightIcons.ruler_dimension_line, label: '여백 (mm)', trailing: SizedBox.shrink()),
        Row(
          spacing: 8,
          children: [
            Tappable(
              onTap: () => _editPageMargin(context, scope, pageLayout, 'top'),
              child: Container(
                padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
                decoration: BoxDecoration(
                  border: Border.all(color: context.colors.borderStrong, width: 0.5),
                  borderRadius: BorderRadius.circular(6),
                ),
                child: Text(
                  '위: ${pageLayout.marginTop.toStringAsFixed(0)}',
                  style: TextStyle(fontSize: 14, color: context.colors.textSubtle),
                ),
              ),
            ),
            Tappable(
              onTap: () => _editPageMargin(context, scope, pageLayout, 'bottom'),
              child: Container(
                padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
                decoration: BoxDecoration(
                  border: Border.all(color: context.colors.borderStrong, width: 0.5),
                  borderRadius: BorderRadius.circular(6),
                ),
                child: Text(
                  '아래: ${pageLayout.marginBottom.toStringAsFixed(0)}',
                  style: TextStyle(fontSize: 14, color: context.colors.textSubtle),
                ),
              ),
            ),
            Tappable(
              onTap: () => _editPageMargin(context, scope, pageLayout, 'left'),
              child: Container(
                padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
                decoration: BoxDecoration(
                  border: Border.all(color: context.colors.borderStrong, width: 0.5),
                  borderRadius: BorderRadius.circular(6),
                ),
                child: Text(
                  '왼쪽: ${pageLayout.marginLeft.toStringAsFixed(0)}',
                  style: TextStyle(fontSize: 14, color: context.colors.textSubtle),
                ),
              ),
            ),
            Tappable(
              onTap: () => _editPageMargin(context, scope, pageLayout, 'right'),
              child: Container(
                padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
                decoration: BoxDecoration(
                  border: Border.all(color: context.colors.borderStrong, width: 0.5),
                  borderRadius: BorderRadius.circular(6),
                ),
                child: Text(
                  '오른쪽: ${pageLayout.marginRight.toStringAsFixed(0)}',
                  style: TextStyle(fontSize: 14, color: context.colors.textSubtle),
                ),
              ),
            ),
          ],
        ),
      ],
    );
  }
}
