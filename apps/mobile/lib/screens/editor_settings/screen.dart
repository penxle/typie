import 'dart:async';

import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:gap/gap.dart';
import 'package:mixpanel_flutter/mixpanel_flutter.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/services/preference.dart';
import 'package:typie/widgets/forms/form.dart';
import 'package:typie/widgets/forms/slider.dart';
import 'package:typie/widgets/forms/switch.dart';
import 'package:typie/widgets/heading.dart';
import 'package:typie/widgets/horizontal_divider.dart';
import 'package:typie/widgets/screen.dart';

@RoutePage()
class EditorSettingsScreen extends HookWidget {
  const EditorSettingsScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final pref = useService<Pref>();
    final mixpanel = useService<Mixpanel>();

    return Screen(
      heading: const Heading(title: '에디터 설정'),
      child: SingleChildScrollView(
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
                                        Text('화면 상단', style: TextStyle(fontSize: 13, color: context.colors.textFaint)),
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
                                        Text('화면 하단', style: TextStyle(fontSize: 13, color: context.colors.textFaint)),
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

                    unawaited(mixpanel.track('toggle_line_highlight', properties: {'enabled': lineHighlightEnabled}));
                  },
                  builder: (context, form) {
                    return _Item(
                      label: '현재 줄 강조',
                      description: '현재 작성 중인 줄을 강조하여 화면에 표시합니다.',
                      trailing: HookFormSwitch(name: 'lineHighlightEnabled', initialValue: pref.lineHighlightEnabled),
                    );
                  },
                ),
              ],
            ),
          ],
        ),
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
        if (trailing != null) trailing!,
      ],
    );

    return Padding(padding: const Pad(all: 16), child: child);
  }
}
