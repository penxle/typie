import 'dart:async';

import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:gap/gap.dart';
import 'package:gql_tristate_value/gql_tristate_value.dart';
import 'package:typie/context/bottom_sheet.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/graphql/__generated__/schema.schema.gql.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/graphql/widget.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/screens/text_replacements/__generated__/create_text_replacement_mutation.req.gql.dart';
import 'package:typie/screens/text_replacements/__generated__/delete_text_replacement_mutation.req.gql.dart';
import 'package:typie/screens/text_replacements/__generated__/move_text_replacement_mutation.req.gql.dart';
import 'package:typie/screens/text_replacements/__generated__/screen_query.data.gql.dart';
import 'package:typie/screens/text_replacements/__generated__/screen_query.req.gql.dart';
import 'package:typie/screens/text_replacements/__generated__/update_text_replacement_mutation.req.gql.dart';
import 'package:typie/widgets/forms/form.dart';
import 'package:typie/widgets/forms/switch.dart';
import 'package:typie/widgets/forms/text_field.dart';
import 'package:typie/widgets/heading.dart';
import 'package:typie/widgets/horizontal_divider.dart';
import 'package:typie/widgets/screen.dart';
import 'package:typie/widgets/tappable.dart';

class _NormalizedItem {
  _NormalizedItem({
    required this.textReplacementId,
    required this.preferenceId,
    required this.match,
    required this.substitute,
    required this.regex,
    required this.preset,
    required this.state,
    required this.order,
    required this.note,
  });

  final String textReplacementId;
  final String? preferenceId;
  final String match;
  final String substitute;
  final bool regex;
  final bool preset;
  final GTextReplacementState state;
  final String? order;
  final String? note;
}

// spell-checker:disable
const _smartQuoteIds = {'TXR0SQUOTEOPEN', 'TXR0SQUOTECLOSE', 'TXR0DQUOTEOPEN', 'TXR0DQUOTECLOSE'};
// spell-checker:enable

_NormalizedItem _normalize(GTextReplacementsScreen_QueryData_me_textReplacements item) {
  return item.when(
    textReplacement: (tr) => _NormalizedItem(
      textReplacementId: tr.id,
      preferenceId: null,
      match: tr.match,
      substitute: tr.substitute,
      regex: tr.regex,
      preset: tr.preset,
      state: GTextReplacementState.ACTIVE,
      order: tr.order,
      note: tr.note,
    ),
    textReplacementPreference: (pref) => _NormalizedItem(
      textReplacementId: pref.textReplacement.id,
      preferenceId: pref.id,
      match: pref.textReplacement.match,
      substitute: pref.textReplacement.substitute,
      regex: pref.textReplacement.regex,
      preset: pref.textReplacement.preset,
      state: pref.state,
      order: pref.order ?? pref.textReplacement.order,
      note: pref.textReplacement.note,
    ),
    orElse: () => _NormalizedItem(
      textReplacementId: '',
      preferenceId: null,
      match: '',
      substitute: '',
      regex: false,
      preset: false,
      state: GTextReplacementState.ACTIVE,
      order: null,
      note: null,
    ),
  );
}

@RoutePage()
class TextReplacementsScreen extends HookWidget {
  const TextReplacementsScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final client = useService<GraphQLClient>();
    final refreshNotifier = useMemoized(RefreshNotifier.new);
    final customItemsRef = useRef<List<_NormalizedItem>>([]);

    return Screen(
      heading: Heading(
        title: '텍스트 대치',
        actions: [
          HeadingAction(
            icon: LucideLightIcons.plus,
            onTap: () async {
              await context.showBottomSheet(
                child: _FormBottomSheet(
                  client: client,
                  refreshNotifier: refreshNotifier,
                  customItems: customItemsRef.value,
                ),
              );
            },
          ),
        ],
      ),
      child: GraphQLOperation(
        operation: GTextReplacementsScreen_QueryReq(),
        refreshNotifier: refreshNotifier,
        builder: (context, client, data) {
          final items = data.me!.textReplacements.map(_normalize).toList();

          final allPresets = items.where((item) => item.preset).toList()
            ..sort((a, b) => (a.order ?? '').compareTo(b.order ?? ''));
          final smartQuoteItems = allPresets.where((item) => _smartQuoteIds.contains(item.textReplacementId)).toList();
          final presets = allPresets.where((item) => !_smartQuoteIds.contains(item.textReplacementId)).toList();
          final smartQuoteAllActive = smartQuoteItems.every((item) => item.state == GTextReplacementState.ACTIVE);
          final customItems = items.where((item) => !item.preset).toList()
            ..sort((a, b) => (a.order ?? '').compareTo(b.order ?? ''));
          customItemsRef.value = customItems;

          return SingleChildScrollView(
            physics: const AlwaysScrollableScrollPhysics(),
            padding: Pad(all: 20, bottom: MediaQuery.paddingOf(context).bottom),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  '입력 중 특정 텍스트를 자동으로 변환해요. v2 에디터에서만 적용돼요.',
                  style: TextStyle(fontSize: 13, color: context.colors.textFaint),
                ),
                const Gap(20),
                _Section(
                  title: '기본 대치',
                  children: [
                    for (int i = 0; i < presets.length; i++) ...[
                      if (i > 0) const _Divider(),
                      _PresetItem(item: presets[i], client: client, refreshNotifier: refreshNotifier),
                    ],
                    if (smartQuoteItems.isNotEmpty) ...[
                      if (presets.isNotEmpty) const _Divider(),
                      _SmartQuoteItem(
                        allActive: smartQuoteAllActive,
                        smartQuoteItems: smartQuoteItems,
                        client: client,
                        refreshNotifier: refreshNotifier,
                      ),
                    ],
                  ],
                ),
                const Gap(24),
                _Section(
                  title: '사용자 대치',
                  description: '위에서부터 순서대로 먼저 매치되는 규칙이 적용돼요.',
                  children: [
                    if (customItems.isEmpty)
                      const Padding(
                        padding: Pad(all: 16),
                        child: Center(child: Text('아직 사용자 대치 규칙이 없어요.', style: TextStyle(fontSize: 13))),
                      )
                    else
                      _CustomItemsList(customItems: customItems, client: client, refreshNotifier: refreshNotifier),
                  ],
                ),
              ],
            ),
          );
        },
      ),
    );
  }
}

class _Section extends StatelessWidget {
  const _Section({required this.title, this.description, required this.children});

  final String title;
  final String? description;
  final List<Widget> children;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          title,
          style: TextStyle(fontSize: 14, fontWeight: FontWeight.w500, color: context.colors.textFaint),
        ),
        if (description != null) ...[
          const Gap(4),
          Text(description!, style: TextStyle(fontSize: 13, color: context.colors.textFaint)),
        ],
        const Gap(8),
        Container(
          decoration: BoxDecoration(
            border: Border.all(color: context.colors.borderStrong),
            borderRadius: BorderRadius.circular(8),
            color: context.colors.surfaceDefault,
          ),
          child: Column(crossAxisAlignment: CrossAxisAlignment.stretch, children: children),
        ),
      ],
    );
  }
}

class _Divider extends StatelessWidget {
  const _Divider();

  @override
  Widget build(BuildContext context) {
    return HorizontalDivider(color: context.colors.borderDefault);
  }
}

class _Item extends StatelessWidget {
  const _Item({required this.label, this.trailing});

  final String label;
  final Widget? trailing;

  @override
  Widget build(BuildContext context) {
    final child = Row(
      children: [
        Expanded(child: Text(label, style: const TextStyle(fontSize: 16))),
        ?trailing,
      ],
    );

    return Padding(padding: const Pad(all: 16), child: child);
  }
}

class _CustomSwitch extends HookWidget {
  const _CustomSwitch({required this.value, required this.onTap});

  final bool value;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final controller = useAnimationController(duration: const Duration(milliseconds: 150));
    final curve = useMemoized(() => CurvedAnimation(parent: controller, curve: Curves.easeInOut));

    useEffect(() {
      if (value) {
        unawaited(controller.forward());
      } else {
        unawaited(controller.reverse());
      }
      return null;
    }, [value]);

    return Tappable(
      onTap: onTap,
      child: Container(
        width: 44,
        height: 24,
        foregroundDecoration: BoxDecoration(
          border: Border.all(color: context.colors.borderStrong),
          borderRadius: BorderRadius.circular(4),
        ),
        child: ClipRRect(
          borderRadius: BorderRadius.circular(4),
          child: Stack(
            children: [
              Row(
                children: [
                  Expanded(child: Container(color: context.colors.accentSuccess)),
                  Expanded(child: Container(color: context.colors.surfaceMuted)),
                ],
              ),
              AnimatedBuilder(
                animation: curve,
                builder: (context, child) {
                  return Align(
                    alignment: Alignment.lerp(Alignment.centerLeft, Alignment.centerRight, curve.value)!,
                    child: Container(
                      width: 24,
                      height: 24,
                      decoration: BoxDecoration(
                        color: context.colors.surfaceDefault,
                        border: Border(
                          left: curve.value > 0
                              ? BorderSide(color: context.colors.borderStrong, width: curve.value)
                              : BorderSide.none,
                          right: curve.value < 1
                              ? BorderSide(color: context.colors.borderStrong, width: 1 - curve.value)
                              : BorderSide.none,
                        ),
                        borderRadius: BorderRadius.circular(4),
                      ),
                      child: child,
                    ),
                  );
                },
                child: const Icon(LucideLightIcons.check, size: 16),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _PresetItem extends StatelessWidget {
  const _PresetItem({required this.item, required this.client, required this.refreshNotifier});

  final _NormalizedItem item;
  final GraphQLClient client;
  final RefreshNotifier refreshNotifier;

  @override
  Widget build(BuildContext context) {
    Future<void> toggle() async {
      final newState = item.state == GTextReplacementState.ACTIVE
          ? GTextReplacementState.DISABLED
          : GTextReplacementState.ACTIVE;
      await client.request(
        GTextReplacementsScreen_UpdateTextReplacement_MutationReq(
          (b) => b
            ..vars.input = GUpdateTextReplacementInput(
              (i) => i
                ..textReplacementId = item.textReplacementId
                ..state = PresentValue(newState),
            ).toBuilder(),
        ),
      );
      refreshNotifier.refresh();
    }

    return Padding(
      padding: const Pad(all: 16),
      child: Row(
        children: [
          Expanded(child: _PresetItemLabel(item: item)),
          _CustomSwitch(value: item.state == GTextReplacementState.ACTIVE, onTap: toggle),
        ],
      ),
    );
  }
}

class _PresetItemLabel extends StatelessWidget {
  const _PresetItemLabel({required this.item});

  final _NormalizedItem item;

  @override
  Widget build(BuildContext context) {
    if (item.note != null && item.note!.isNotEmpty) {
      return Row(
        children: [
          Flexible(child: Text(item.note!, style: const TextStyle(fontSize: 16))),
          if (item.regex) ...[const Gap(6), _RegexBadge()],
        ],
      );
    }

    return Row(
      children: [
        Flexible(
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              Flexible(
                child: Container(
                  padding: const Pad(horizontal: 6, vertical: 2),
                  decoration: BoxDecoration(color: context.colors.surfaceMuted, borderRadius: BorderRadius.circular(4)),
                  child: Text(
                    item.match,
                    style: const TextStyle(fontSize: 12, fontFamily: 'monospace'),
                    overflow: TextOverflow.ellipsis,
                  ),
                ),
              ),
              Padding(
                padding: const Pad(horizontal: 6),
                child: Text('→', style: TextStyle(fontSize: 13, color: context.colors.textFaint)),
              ),
              Flexible(
                child: Container(
                  padding: const Pad(horizontal: 6, vertical: 2),
                  decoration: BoxDecoration(color: context.colors.surfaceMuted, borderRadius: BorderRadius.circular(4)),
                  child: Text(
                    item.substitute,
                    style: const TextStyle(fontSize: 12, fontFamily: 'monospace'),
                    overflow: TextOverflow.ellipsis,
                  ),
                ),
              ),
            ],
          ),
        ),
        if (item.regex) ...[const Gap(6), _RegexBadge()],
      ],
    );
  }
}

class _RegexBadge extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const Pad(horizontal: 6, vertical: 2),
      decoration: BoxDecoration(
        border: Border.all(color: context.colors.borderDefault),
        borderRadius: BorderRadius.circular(99),
      ),
      child: Text(
        '정규식',
        style: TextStyle(fontSize: 11, fontWeight: FontWeight.w500, color: context.colors.textFaint),
      ),
    );
  }
}

class _SmartQuoteItem extends StatelessWidget {
  const _SmartQuoteItem({
    required this.allActive,
    required this.smartQuoteItems,
    required this.client,
    required this.refreshNotifier,
  });

  final bool allActive;
  final List<_NormalizedItem> smartQuoteItems;
  final GraphQLClient client;
  final RefreshNotifier refreshNotifier;

  @override
  Widget build(BuildContext context) {
    Future<void> toggle() async {
      final newState = allActive ? GTextReplacementState.DISABLED : GTextReplacementState.ACTIVE;
      for (final item in smartQuoteItems) {
        await client.request(
          GTextReplacementsScreen_UpdateTextReplacement_MutationReq(
            (b) => b
              ..vars.input = GUpdateTextReplacementInput(
                (i) => i
                  ..textReplacementId = item.textReplacementId
                  ..state = PresentValue(newState),
              ).toBuilder(),
          ),
        );
      }
      refreshNotifier.refresh();
    }

    return _Item(
      label: '곧은따옴표를 둥근따옴표로',
      trailing: _CustomSwitch(value: allActive, onTap: toggle),
    );
  }
}

class _CustomItemsList extends HookWidget {
  const _CustomItemsList({required this.customItems, required this.client, required this.refreshNotifier});

  final List<_NormalizedItem> customItems;
  final GraphQLClient client;
  final RefreshNotifier refreshNotifier;

  @override
  Widget build(BuildContext context) {
    final optimisticOrder = useRef<List<String>?>(null);
    final forceRebuild = useState(0);

    if (optimisticOrder.value != null) {
      final serverIds = customItems.map((e) => e.textReplacementId).toList();
      if (listEquals(serverIds, optimisticOrder.value)) {
        optimisticOrder.value = null;
      }
    }

    List<_NormalizedItem> displayItems;
    if (optimisticOrder.value != null) {
      final itemMap = {for (final item in customItems) item.textReplacementId: item};
      displayItems = optimisticOrder.value!.where(itemMap.containsKey).map((id) => itemMap[id]!).toList();
    } else {
      displayItems = customItems;
    }

    return ReorderableListView.builder(
      shrinkWrap: true,
      physics: const NeverScrollableScrollPhysics(),
      buildDefaultDragHandles: false,
      itemCount: displayItems.length,
      proxyDecorator: (child, index, animation) {
        return AnimatedBuilder(
          animation: animation,
          builder: (context, child) {
            final elevation = Tween<double>(begin: 0, end: 4).animate(animation).value;
            return Material(
              elevation: elevation,
              color: context.colors.surfaceDefault,
              borderRadius: BorderRadius.circular(8),
              child: child,
            );
          },
          child: child,
        );
      },
      onReorder: (oldIndex, newIndex) async {
        if (oldIndex < newIndex) {
          newIndex -= 1;
        }
        if (oldIndex == newIndex) {
          return;
        }

        final reordered = List<_NormalizedItem>.from(displayItems);
        final item = reordered.removeAt(oldIndex);
        reordered.insert(newIndex, item);

        optimisticOrder.value = reordered.map((e) => e.textReplacementId).toList();
        forceRebuild.value++;

        final lowerOrder = newIndex > 0 ? reordered[newIndex - 1].order : null;
        final upperOrder = newIndex < reordered.length - 1 ? reordered[newIndex + 1].order : null;

        await client.request(
          GTextReplacementsScreen_MoveTextReplacement_MutationReq(
            (b) => b
              ..vars.input = GMoveTextReplacementInput((i) {
                i.textReplacementId = item.textReplacementId;
                if (lowerOrder != null) {
                  i.lowerOrder = PresentValue(lowerOrder);
                }
                if (upperOrder != null) {
                  i.upperOrder = PresentValue(upperOrder);
                }
              }).toBuilder(),
          ),
        );
        refreshNotifier.refresh();
      },
      itemBuilder: (context, index) {
        final item = displayItems[index];

        Future<void> toggle() async {
          final newState = item.state == GTextReplacementState.ACTIVE
              ? GTextReplacementState.DISABLED
              : GTextReplacementState.ACTIVE;
          await client.request(
            GTextReplacementsScreen_UpdateTextReplacement_MutationReq(
              (b) => b
                ..vars.input = GUpdateTextReplacementInput(
                  (i) => i
                    ..textReplacementId = item.textReplacementId
                    ..state = PresentValue(newState),
                ).toBuilder(),
            ),
          );
          refreshNotifier.refresh();
        }

        return Column(
          key: ValueKey(item.textReplacementId),
          children: [
            if (index > 0) const _Divider(),
            Padding(
              padding: const Pad(horizontal: 16, vertical: 12),
              child: Row(
                children: [
                  ReorderableDragStartListener(
                    index: index,
                    child: Padding(
                      padding: const Pad(right: 8),
                      child: Icon(LucideLightIcons.grip_vertical, size: 16, color: context.colors.textFaint),
                    ),
                  ),
                  Container(
                    padding: const Pad(horizontal: 6, vertical: 2),
                    decoration: BoxDecoration(
                      color: context.colors.surfaceMuted,
                      borderRadius: BorderRadius.circular(4),
                    ),
                    child: Text(
                      '${index + 1}',
                      style: TextStyle(
                        fontSize: 11,
                        color: context.colors.textFaint,
                        fontFeatures: const [FontFeature.tabularFigures()],
                      ),
                    ),
                  ),
                  const Gap(8),
                  Expanded(child: _PresetItemLabel(item: item)),
                  const Gap(8),
                  _CustomSwitch(value: item.state == GTextReplacementState.ACTIVE, onTap: toggle),
                  Tappable(
                    onTap: () async {
                      await context.showBottomSheet(
                        child: _FormBottomSheet(
                          client: client,
                          refreshNotifier: refreshNotifier,
                          editingItem: item,
                          customItems: displayItems,
                        ),
                      );
                    },
                    child: Padding(
                      padding: const Pad(left: 4),
                      child: Icon(LucideLightIcons.chevron_right, size: 20, color: context.colors.textFaint),
                    ),
                  ),
                ],
              ),
            ),
          ],
        );
      },
    );
  }
}

class _FormBottomSheet extends HookWidget {
  const _FormBottomSheet({
    required this.client,
    required this.refreshNotifier,
    required this.customItems,
    this.editingItem,
  });

  final GraphQLClient client;
  final RefreshNotifier refreshNotifier;
  final _NormalizedItem? editingItem;
  final List<_NormalizedItem> customItems;

  @override
  Widget build(BuildContext context) {
    final isEditing = editingItem != null;
    final form = useHookForm();
    final errorText = useState<String?>(null);

    bool validate() {
      final match = (form.data['match'] as String?)?.trim() ?? '';
      final substitute = (form.data['substitute'] as String?)?.trim() ?? '';
      final regex = form.data['regex'] as bool? ?? false;

      if (match.isEmpty) {
        errorText.value = '찾을 텍스트를 입력해 주세요.';
        return false;
      }
      if (substitute.isEmpty) {
        errorText.value = '삽입할 텍스트를 입력해 주세요.';
        return false;
      }
      if (match == substitute) {
        errorText.value = '찾을 텍스트와 삽입할 텍스트가 같아요.';
        return false;
      }
      if (regex) {
        try {
          RegExp(match);
        } catch (_) {
          errorText.value = '유효하지 않은 정규식이에요.';
          return false;
        }
      }
      errorText.value = null;
      return true;
    }

    Future<void> handleSave() async {
      if (!validate()) {
        return;
      }

      final match = form.data['match'] as String;
      final substitute = form.data['substitute'] as String;
      final regex = form.data['regex'] as bool? ?? false;
      final note = form.data['note'] as String? ?? '';

      if (isEditing) {
        await client.request(
          GTextReplacementsScreen_UpdateTextReplacement_MutationReq(
            (b) => b
              ..vars.input = GUpdateTextReplacementInput(
                (i) => i
                  ..textReplacementId = editingItem!.textReplacementId
                  ..match = PresentValue(match)
                  ..substitute = PresentValue(substitute)
                  ..regex = PresentValue(regex)
                  ..note = PresentValue(note),
              ).toBuilder(),
          ),
        );
      } else {
        final lastOrder = customItems.isNotEmpty ? customItems.last.order : null;
        await client.request(
          GTextReplacementsScreen_CreateTextReplacement_MutationReq((b) {
            b.vars.input = GCreateTextReplacementInput((i) {
              i
                ..match = match
                ..substitute = substitute
                ..regex = PresentValue(regex)
                ..note = PresentValue(note);
              if (lastOrder != null) {
                i.lowerOrder = PresentValue(lastOrder);
              }
            }).toBuilder();
          }),
        );
      }

      refreshNotifier.refresh();
      if (context.mounted) {
        await context.router.maybePop();
      }
    }

    Future<void> handleDelete() async {
      if (!isEditing) {
        return;
      }

      await context.showBottomSheet(
        child: ConfirmBottomSheet(
          title: '대치 규칙 삭제',
          message: '"${editingItem!.match} → ${editingItem!.substitute}" 규칙을 삭제하시겠어요?',
          confirmText: '삭제',
          onConfirm: () async {
            await client.request(
              GTextReplacementsScreen_DeleteTextReplacement_MutationReq(
                (b) => b
                  ..vars.input = GDeleteTextReplacementInput(
                    (i) => i..textReplacementId = editingItem!.textReplacementId,
                  ).toBuilder(),
              ),
            );
            refreshNotifier.refresh();
            if (context.mounted) {
              await context.router.maybePop();
            }
          },
        ),
      );
    }

    return AppBottomSheet(
      padding: const Pad(horizontal: 20),
      child: HookForm(
        form: form,
        builder: (context, form) {
          return Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            spacing: 16,
            children: [
              Text(
                isEditing ? '대치 규칙 수정' : '대치 규칙 추가',
                style: const TextStyle(fontSize: 18, fontWeight: FontWeight.w700),
              ),
              HookFormTextField(
                name: 'match',
                label: '찾을 텍스트',
                placeholder: '찾을 텍스트를 입력해 주세요',
                initialValue: editingItem?.match,
                autofocus: !isEditing,
                onChanged: (_) => errorText.value = null,
              ),
              HookFormTextField(
                name: 'substitute',
                label: '삽입할 텍스트',
                placeholder: '삽입할 텍스트를 입력해 주세요',
                initialValue: editingItem?.substitute,
                onChanged: (_) => errorText.value = null,
              ),
              HookFormTextField(name: 'note', label: '설명', placeholder: '설명 (선택)', initialValue: editingItem?.note),
              Row(
                children: [
                  const Expanded(child: Text('정규식', style: TextStyle(fontSize: 16))),
                  HookFormSwitch(name: 'regex', initialValue: editingItem?.regex ?? false),
                ],
              ),
              if (errorText.value != null)
                Text(errorText.value!, style: TextStyle(fontSize: 13, color: context.colors.accentDanger)),
              const Gap(8),
              Row(
                spacing: 8,
                children: [
                  if (isEditing)
                    Expanded(
                      child: Tappable(
                        onTap: handleDelete,
                        child: Container(
                          alignment: Alignment.center,
                          decoration: BoxDecoration(
                            color: context.colors.surfaceMuted,
                            borderRadius: BorderRadius.circular(8),
                          ),
                          padding: const Pad(vertical: 16),
                          child: Text(
                            '삭제',
                            style: TextStyle(
                              fontSize: 16,
                              fontWeight: FontWeight.w600,
                              color: context.colors.accentDanger,
                            ),
                          ),
                        ),
                      ),
                    ),
                  Expanded(
                    child: Tappable(
                      onTap: () async {
                        await context.router.maybePop();
                      },
                      child: Container(
                        alignment: Alignment.center,
                        decoration: BoxDecoration(
                          color: context.colors.surfaceMuted,
                          borderRadius: BorderRadius.circular(8),
                        ),
                        padding: const Pad(vertical: 16),
                        child: const Text('취소', style: TextStyle(fontSize: 16, fontWeight: FontWeight.w600)),
                      ),
                    ),
                  ),
                  Expanded(
                    child: Tappable(
                      onTap: handleSave,
                      child: Container(
                        alignment: Alignment.center,
                        decoration: BoxDecoration(
                          color: context.colors.surfaceInverse,
                          borderRadius: BorderRadius.circular(8),
                        ),
                        padding: const Pad(vertical: 16),
                        child: Text(
                          '저장',
                          style: TextStyle(
                            fontSize: 16,
                            fontWeight: FontWeight.w600,
                            color: context.colors.textInverse,
                          ),
                        ),
                      ),
                    ),
                  ),
                ],
              ),
            ],
          );
        },
      ),
    );
  }
}
