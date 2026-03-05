import 'dart:async';

import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:built_value/json_object.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:gap/gap.dart';
import 'package:mixpanel_flutter/mixpanel_flutter.dart';
import 'package:typie/context/bottom_sheet.dart';
import 'package:typie/context/loader.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/graphql/widget.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/screens/editor_settings/__generated__/screen_query.req.gql.dart';
import 'package:typie/screens/editor_settings/__generated__/update_preferences_mutation.req.gql.dart';
import 'package:typie/services/preference.dart';
import 'package:typie/widgets/forms/form.dart';
import 'package:typie/widgets/forms/slider.dart';
import 'package:typie/widgets/forms/switch.dart';
import 'package:typie/widgets/heading.dart';
import 'package:typie/widgets/horizontal_divider.dart';
import 'package:typie/widgets/screen.dart';
import 'package:typie/widgets/tappable.dart';

@RoutePage()
class EditorSettingsScreen extends HookWidget {
  const EditorSettingsScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final pref = useService<Pref>();
    final mixpanel = useService<Mixpanel>();
    final isUpdatingAiOptIn = useState(false);
    const aiOptInLoaderDelay = Duration(milliseconds: 150);

    return Screen(
      heading: const Heading(title: '에디터 설정'),
      child: GraphQLOperation(
        operation: GEditorSettingsScreen_QueryReq(),
        builder: (context, client, data) {
          final aiOptIn = data.me!.preferences.asMap['aiOptIn'] as bool? ?? false;

          Future<void> toggleAiOptIn() async {
            if (isUpdatingAiOptIn.value) {
              return;
            }

            if (aiOptIn) {
              isUpdatingAiOptIn.value = true;
              try {
                await context.runWithLoader(() async {
                  await client.request(
                    GEditorSettingsScreen_UpdatePreferences_MutationReq(
                      (b) => b..vars.input.value = JsonObject({'aiOptIn': false}),
                    ),
                  );
                }, showDelay: aiOptInLoaderDelay);
                unawaited(mixpanel.track('ai_opt_in', properties: {'enabled': false}));
              } finally {
                isUpdatingAiOptIn.value = false;
              }
            } else {
              await context.showBottomSheet(
                child: ConfirmBottomSheet(
                  title: 'AI 기능을 활성화하시겠어요?',
                  confirmText: '활성화',
                  onConfirm: () async {
                    if (isUpdatingAiOptIn.value) {
                      return;
                    }

                    isUpdatingAiOptIn.value = true;
                    try {
                      await context.runWithLoader(() async {
                        await client.request(
                          GEditorSettingsScreen_UpdatePreferences_MutationReq(
                            (b) => b..vars.input.value = JsonObject({'aiOptIn': true}),
                          ),
                        );
                      }, showDelay: aiOptInLoaderDelay);
                      unawaited(mixpanel.track('ai_opt_in', properties: {'enabled': true}));
                    } finally {
                      isUpdatingAiOptIn.value = false;
                    }
                  },
                  child: const _AiOptInNotice(),
                ),
              );
            }
          }

          return SingleChildScrollView(
            physics: const AlwaysScrollableScrollPhysics(),
            padding: Pad(all: 20, bottom: MediaQuery.paddingOf(context).bottom),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              spacing: 24,
              children: [
                _Section(
                  title: '작성 위치',
                  children: [
                    HookForm(
                      submitMode: HookFormSubmitMode.onChange,
                      onSubmit: (form) async {
                        final typewriterEnabled = form.data['typewriterEnabled'] as bool;
                        pref.typewriterEnabled = typewriterEnabled;

                        unawaited(mixpanel.track('toggle_typewriter', properties: {'enabled': typewriterEnabled}));
                      },
                      builder: (context, form) {
                        return Column(
                          children: [
                            _Item(
                              label: '타자기 모드',
                              description: '현재 작성 중인 줄을 항상 화면의 특정 위치에 고정합니다.',
                              trailing: HookFormSwitch(name: 'typewriterEnabled', initialValue: pref.typewriterEnabled),
                            ),
                            if (pref.typewriterEnabled) ...[
                              const _Divider(),
                              HookForm(
                                submitMode: HookFormSubmitMode.onChange,
                                onSubmit: (form) async {
                                  final position = form.data['typewriterPosition'] as double;
                                  pref.typewriterPosition = position;

                                  unawaited(
                                    mixpanel.track(
                                      'change_typewriter_position',
                                      properties: {'position': (position * 100).round()},
                                    ),
                                  );
                                },
                                builder: (context, form) {
                                  return Padding(
                                    padding: const Pad(horizontal: 16, vertical: 8),
                                    child: Column(
                                      crossAxisAlignment: CrossAxisAlignment.start,
                                      children: [
                                        const Text('고정 위치', style: TextStyle(fontSize: 14)),
                                        const Gap(4),
                                        Text(
                                          '현재 작성 중인 줄이 고정될 화면상의 위치를 설정합니다.',
                                          style: TextStyle(fontSize: 13, color: context.colors.textFaint),
                                        ),
                                        const Gap(12),
                                        Row(
                                          spacing: 12,
                                          children: [
                                            Text(
                                              '화면 상단',
                                              style: TextStyle(fontSize: 13, color: context.colors.textFaint),
                                            ),
                                            Expanded(
                                              child: Padding(
                                                padding: const Pad(horizontal: 4),
                                                child: HookFormSlider(
                                                  name: 'typewriterPosition',
                                                  min: 0,
                                                  max: 1,
                                                  step: 0.05,
                                                  initialValue: pref.typewriterPosition,
                                                ),
                                              ),
                                            ),
                                            Text(
                                              '화면 하단',
                                              style: TextStyle(fontSize: 13, color: context.colors.textFaint),
                                            ),
                                          ],
                                        ),
                                      ],
                                    ),
                                  );
                                },
                              ),
                            ],
                          ],
                        );
                      },
                    ),
                  ],
                ),
                _Section(
                  title: '표시 설정',
                  children: [
                    HookForm(
                      submitMode: HookFormSubmitMode.onChange,
                      onSubmit: (form) async {
                        final lineHighlightEnabled = form.data['lineHighlightEnabled'] as bool;
                        pref.lineHighlightEnabled = lineHighlightEnabled;

                        unawaited(
                          mixpanel.track('toggle_line_highlight', properties: {'enabled': lineHighlightEnabled}),
                        );
                      },
                      builder: (context, form) {
                        return _Item(
                          label: '현재 줄 강조',
                          description: '현재 작성 중인 줄을 강조하여 화면에 표시합니다.',
                          trailing: HookFormSwitch(
                            name: 'lineHighlightEnabled',
                            initialValue: pref.lineHighlightEnabled,
                          ),
                        );
                      },
                    ),
                  ],
                ),
                _Section(
                  title: '편집 설정',
                  children: [
                    HookForm(
                      submitMode: HookFormSubmitMode.onChange,
                      onSubmit: (form) async {
                        final autoSurroundEnabled = form.data['autoSurroundEnabled'] as bool;
                        pref.autoSurroundEnabled = autoSurroundEnabled;

                        unawaited(mixpanel.track('toggle_auto_surround', properties: {'enabled': autoSurroundEnabled}));
                      },
                      builder: (context, form) {
                        return _Item(
                          label: '선택 영역 둘러싸기',
                          description: '따옴표나 괄호를 입력하면 선택 영역을 둘러쌉니다.',
                          trailing: HookFormSwitch(name: 'autoSurroundEnabled', initialValue: pref.autoSurroundEnabled),
                        );
                      },
                    ),
                  ],
                ),
                _Section(
                  title: '위젯 설정',
                  children: [
                    HookForm(
                      submitMode: HookFormSubmitMode.onChange,
                      onSubmit: (form) async {
                        final characterCountFloatingEnabled = form.data['characterCountFloatingEnabled'] as bool;
                        pref.characterCountFloatingEnabled = characterCountFloatingEnabled;

                        unawaited(
                          mixpanel.track(
                            'toggle_character_count_floating',
                            properties: {'enabled': characterCountFloatingEnabled},
                          ),
                        );
                      },
                      builder: (context, form) {
                        return Column(
                          children: [
                            _Item(
                              label: '글자 수 위젯',
                              description: '에디터에서 글자 수를 표시합니다.',
                              trailing: HookFormSwitch(
                                name: 'characterCountFloatingEnabled',
                                initialValue: pref.characterCountFloatingEnabled,
                              ),
                            ),
                            if (pref.characterCountFloatingEnabled) ...[
                              const _Divider(),
                              HookForm(
                                submitMode: HookFormSubmitMode.onChange,
                                onSubmit: (form) async {
                                  final widgetAutoFadeEnabled = form.data['widgetAutoFadeEnabled'] as bool;
                                  pref.widgetAutoFadeEnabled = widgetAutoFadeEnabled;

                                  unawaited(
                                    mixpanel.track(
                                      'toggle_widget_auto_fade',
                                      properties: {'enabled': widgetAutoFadeEnabled},
                                    ),
                                  );
                                },
                                builder: (context, form) {
                                  return _Item(
                                    label: '위젯 자동 페이드 인/아웃',
                                    description: '타이핑, 스크롤 시 위젯이 잠시 사라집니다.',
                                    trailing: HookFormSwitch(
                                      name: 'widgetAutoFadeEnabled',
                                      initialValue: pref.widgetAutoFadeEnabled,
                                    ),
                                  );
                                },
                              ),
                            ],
                          ],
                        );
                      },
                    ),
                  ],
                ),
                _Section(
                  title: 'AI 설정',
                  children: [
                    _Item(
                      label: 'AI 기능 활성화',
                      description: '활성화하면 AI 피드백 등 타이피가 제공하는 AI 기능을 사용할 수 있어요.',
                      trailing: _CustomSwitch(value: aiOptIn, onTap: toggleAiOptIn),
                    ),
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
  const _Section({required this.title, required this.children});

  final String title;
  final List<Widget> children;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      spacing: 8,
      children: [
        Text(
          title,
          style: TextStyle(fontSize: 14, fontWeight: FontWeight.w500, color: context.colors.textFaint),
        ),
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
  const _Item({required this.label, this.description, this.trailing});

  final String label;
  final String? description;
  final Widget? trailing;

  @override
  Widget build(BuildContext context) {
    final child = Row(
      children: [
        Expanded(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(label, style: const TextStyle(fontSize: 16)),
              if (description != null) ...[
                const Gap(4),
                Text(description!, style: TextStyle(fontSize: 13, color: context.colors.textFaint)),
              ],
            ],
          ),
        ),
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

class _AiOptInNotice extends StatelessWidget {
  const _AiOptInNotice();

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          '타이피는 사용자의 프라이버시를 최우선으로 생각해요. 사용자가 작성한 글은 어떠한 경우에도 AI 모델 학습에 사용되지 않아요.',
          style: TextStyle(fontSize: 14, color: context.colors.textFaint),
        ),
        const Gap(16),
        Container(
          padding: const Pad(all: 12),
          decoration: BoxDecoration(color: context.colors.surfaceMuted, borderRadius: BorderRadius.circular(8)),
          child: const Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            spacing: 8,
            children: [
              _AiOptInNoticeItem(title: '학습 금지', description: '사용자의 글은 AI 모델 학습이나 개선에 절대 사용되지 않아요.'),
              _AiOptInNoticeItem(title: '요청 시에만', description: '사용자가 요청하지 않는 한 타이피가 임의로 AI를 사용하지 않아요.'),
              _AiOptInNoticeItem(title: '투명한 처리', description: 'AI가 언제, 어떻게 사용되는지 사용자가 항상 알 수 있어요.'),
              _AiOptInNoticeItem(title: '완전한 통제', description: 'AI 기능은 언제든 끌 수 있고, 비활성화하면 어떤 AI 처리도 일어나지 않아요.'),
              _AiOptInNoticeItem(title: '권리 보장', description: '타이피는 사용자 창작물에 대한 어떤 권리도 주장하지 않아요.'),
            ],
          ),
        ),
      ],
    );
  }
}

class _AiOptInNoticeItem extends StatelessWidget {
  const _AiOptInNoticeItem({required this.title, required this.description});

  final String title;
  final String description;

  @override
  Widget build(BuildContext context) {
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text('• ', style: TextStyle(fontSize: 13, color: context.colors.textFaint)),
        Expanded(
          child: RichText(
            text: TextSpan(
              style: TextStyle(fontSize: 13, color: context.colors.textFaint),
              children: [
                TextSpan(
                  text: '$title: ',
                  style: const TextStyle(fontWeight: FontWeight.w600),
                ),
                TextSpan(text: description),
              ],
            ),
          ),
        ),
      ],
    );
  }
}
