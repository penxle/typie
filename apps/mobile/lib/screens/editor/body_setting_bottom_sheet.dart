import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:gap/gap.dart';
import 'package:mixpanel_flutter/mixpanel_flutter.dart';
import 'package:typie/context/bottom_sheet.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/icons/lucide_lab.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/screens/editor/schema.dart';
import 'package:typie/screens/editor/scope.dart';
import 'package:typie/widgets/forms/form.dart';
import 'package:typie/widgets/forms/select.dart';
import 'package:typie/widgets/horizontal_divider.dart';

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
          await context.showBottomSheet(
            child: _IncompatibleBlocksBottomSheet(
              blocks: blocks.map(getBlockName).toList(),
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
            await scope.webViewController.value?.emitEvent('setPageLayout', {'preset': dirtyData['pageSize']});
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
                _PageSizeSection(pageLayout: pageLayout),
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

class _PageSizeSection extends HookWidget {
  const _PageSizeSection({required this.pageLayout});

  final PageLayout pageLayout;

  String _getPageSizePreset(PageLayout layout) {
    if (layout.width == 210 && layout.height == 297) {
      return 'a4';
    }
    if (layout.width == 148 && layout.height == 210) {
      return 'a5';
    }
    if (layout.width == 176 && layout.height == 250) {
      return 'b5';
    }
    if (layout.width == 125 && layout.height == 176) {
      return 'b6';
    }
    return 'custom';
  }

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
        Padding(
          padding: const EdgeInsets.only(left: 14),
          child: Row(
            children: [
              Text(
                '${pageLayout.width.toStringAsFixed(0)} × ${pageLayout.height.toStringAsFixed(0)}mm',
                style: TextStyle(fontSize: 14, color: context.colors.textSubtle),
              ),
            ],
          ),
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
        const _Option(icon: LucideLightIcons.ruler_dimension_line, label: '여백', trailing: SizedBox.shrink()),
        Padding(
          padding: const EdgeInsets.only(left: 14),
          child: Row(
            spacing: 8,
            children: [
              Text(
                '위: ${pageLayout.marginTop.toStringAsFixed(0)}mm',
                style: TextStyle(fontSize: 14, color: context.colors.textSubtle),
              ),
              Text(
                '아래: ${pageLayout.marginBottom.toStringAsFixed(0)}mm',
                style: TextStyle(fontSize: 14, color: context.colors.textSubtle),
              ),
              Text(
                '왼쪽: ${pageLayout.marginLeft.toStringAsFixed(0)}mm',
                style: TextStyle(fontSize: 14, color: context.colors.textSubtle),
              ),
              Text(
                '오른쪽: ${pageLayout.marginRight.toStringAsFixed(0)}mm',
                style: TextStyle(fontSize: 14, color: context.colors.textSubtle),
              ),
            ],
          ),
        ),
      ],
    );
  }
}

class _IncompatibleBlocksBottomSheet extends StatelessWidget {
  const _IncompatibleBlocksBottomSheet({required this.blocks, required this.onConfirm, required this.onCancel});

  final List<String> blocks;
  final VoidCallback onConfirm;
  final VoidCallback onCancel;

  @override
  Widget build(BuildContext context) {
    return AppBottomSheet(
      padding: const Pad(horizontal: 20),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        spacing: 16,
        children: [
          Text(
            '페이지 모드 전환',
            style: TextStyle(fontSize: 17, fontWeight: FontWeight.w700, color: context.colors.textDefault),
          ),
          Text(
            '페이지 모드에서는 일부 블록을 지원하지 않아요.\n다음 블록들을 해제해서 일반 문단으로 변환할까요?',
            style: TextStyle(fontSize: 15, color: context.colors.textSubtle),
          ),
          Container(
            padding: const EdgeInsets.all(12),
            decoration: BoxDecoration(color: context.colors.surfaceSubtle, borderRadius: BorderRadius.circular(6)),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              spacing: 8,
              children: blocks
                  .map(
                    (block) => Row(
                      children: [
                        Icon(_getBlockIcon(block), size: 16, color: context.colors.textDefault),
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
          Row(
            spacing: 8,
            children: [
              Expanded(
                child: GestureDetector(
                  onTap: () {
                    Navigator.of(context).pop();
                    onCancel();
                  },
                  child: Container(
                    alignment: Alignment.center,
                    decoration: BoxDecoration(
                      color: context.colors.surfaceMuted,
                      borderRadius: BorderRadius.circular(999),
                    ),
                    padding: const EdgeInsets.symmetric(vertical: 12),
                    child: const Text('취소', style: TextStyle(fontSize: 16, fontWeight: FontWeight.w600)),
                  ),
                ),
              ),
              Expanded(
                child: GestureDetector(
                  onTap: () {
                    Navigator.of(context).pop();
                    onConfirm();
                  },
                  child: Container(
                    alignment: Alignment.center,
                    decoration: BoxDecoration(
                      color: context.colors.surfaceInverse,
                      borderRadius: BorderRadius.circular(999),
                    ),
                    padding: const EdgeInsets.symmetric(vertical: 12),
                    child: Text(
                      '모두 해제하고 전환',
                      style: TextStyle(fontSize: 16, fontWeight: FontWeight.w600, color: context.colors.textInverse),
                    ),
                  ),
                ),
              ),
            ],
          ),
        ],
      ),
    );
  }

  IconData _getBlockIcon(String blockName) {
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
}
