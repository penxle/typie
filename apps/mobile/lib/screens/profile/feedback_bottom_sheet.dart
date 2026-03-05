import 'dart:io';

import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:gap/gap.dart';
import 'package:gql_tristate_value/gql_tristate_value.dart';
import 'package:package_info_plus/package_info_plus.dart';
import 'package:typie/context/bottom_sheet.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/context/toast.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/screens/profile/__generated__/submit_feedback_mutation.req.gql.dart';
import 'package:typie/widgets/tappable.dart';

class FeedbackBottomSheet extends HookWidget {
  const FeedbackBottomSheet({required this.client, super.key});

  final GraphQLClient client;

  @override
  Widget build(BuildContext context) {
    final topic = useState<String?>(null);
    final mood = useState<String?>(null);
    final submitting = useState(false);
    final contentController = useTextEditingController();
    final content = useState('');

    useEffect(() {
      void listener() {
        content.value = contentController.text;
      }

      contentController.addListener(listener);
      return () => contentController.removeListener(listener);
    }, [contentController]);

    final canSubmit = topic.value != null && content.value.trim().isNotEmpty && !submitting.value;

    Future<void> handleSubmit() async {
      if (submitting.value) {
        return;
      }

      if (topic.value == null) {
        context.toast(ToastType.error, '주제를 선택해주세요.');
        return;
      }

      final trimmed = content.value.trim();
      if (trimmed.isEmpty) {
        context.toast(ToastType.error, '내용을 입력해주세요.');
        return;
      }

      submitting.value = true;

      try {
        final packageInfo = await PackageInfo.fromPlatform();
        final os = switch (Platform.operatingSystem) {
          'ios' => 'iOS',
          'android' => 'Android',
          _ => Platform.operatingSystem,
        };
        final appVersion = '${packageInfo.version} (${packageInfo.buildNumber})';
        final url = '$os | $appVersion';

        await client.request(
          GProfileScreen_SubmitFeedback_MutationReq(
            (b) => b
              ..vars.input.topic = Value.present(topic.value)
              ..vars.input.content = trimmed
              ..vars.input.mood = Value.present(mood.value)
              ..vars.input.url = Value.present(url),
          ),
        );

        if (context.mounted) {
          context.toast(ToastType.success, '피드백을 보냈어요. 감사해요!');
          await context.router.maybePop();
        }
      } catch (_) {
        if (context.mounted) {
          context.toast(ToastType.error, '의견 전송에 실패했어요. 잠시 후 다시 시도해주세요.');
        }
      } finally {
        if (context.mounted) {
          submitting.value = false;
        }
      }
    }

    return AppBottomSheet(
      padding: const Pad(horizontal: 20),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        spacing: 12,
        children: [
          Text(
            '의견 보내기',
            style: TextStyle(fontSize: 16, fontWeight: FontWeight.w700, color: context.colors.textDefault),
          ),
          Wrap(
            spacing: 8,
            runSpacing: 8,
            children: feedbackTopics
                .map(
                  (item) => _SelectionChip(
                    selected: topic.value == item.value,
                    label: item.label,
                    onTap: () {
                      topic.value = item.value;
                    },
                  ),
                )
                .toList(),
          ),
          TextField(
            controller: contentController,
            minLines: 4,
            maxLines: 6,
            textInputAction: TextInputAction.newline,
            style: TextStyle(fontSize: 14, color: context.colors.textDefault),
            decoration: InputDecoration(
              hintText: '칭찬도, 불만도, 아이디어도 다 좋아요!',
              hintStyle: TextStyle(fontSize: 14, color: context.colors.textFaint),
              contentPadding: const Pad(all: 12),
              filled: true,
              fillColor: context.colors.surfaceDefault,
              border: OutlineInputBorder(
                borderRadius: BorderRadius.circular(8),
                borderSide: BorderSide(color: context.colors.borderDefault),
              ),
              enabledBorder: OutlineInputBorder(
                borderRadius: BorderRadius.circular(8),
                borderSide: BorderSide(color: context.colors.borderDefault),
              ),
              focusedBorder: OutlineInputBorder(
                borderRadius: BorderRadius.circular(8),
                borderSide: BorderSide(color: context.colors.accentBrand),
              ),
            ),
          ),
          Row(
            children: [
              Row(
                spacing: 4,
                children: feedbackMoods
                    .map(
                      (item) => Tappable(
                        onTap: () {
                          mood.value = mood.value == item.value ? null : item.value;
                        },
                        child: Container(
                          width: 34,
                          height: 34,
                          decoration: BoxDecoration(
                            color: mood.value == item.value ? context.colors.accentBrandSubtle : Colors.transparent,
                            borderRadius: BorderRadius.circular(999),
                            border: Border.all(
                              color: mood.value == item.value
                                  ? context.colors.accentBrand
                                  : context.colors.borderDefault,
                            ),
                          ),
                          child: Icon(
                            item.icon,
                            size: 18,
                            color: mood.value == item.value ? context.colors.accentBrand : context.colors.textFaint,
                          ),
                        ),
                      ),
                    )
                    .toList(),
              ),
              const Spacer(),
              Tappable(
                onTap: handleSubmit,
                child: Container(
                  padding: const Pad(horizontal: 14, vertical: 10),
                  decoration: BoxDecoration(
                    color: canSubmit ? context.colors.accentBrand : context.colors.surfaceMuted,
                    borderRadius: BorderRadius.circular(8),
                  ),
                  child: Text(
                    submitting.value ? '보내는 중...' : '보내기',
                    style: TextStyle(
                      fontSize: 14,
                      fontWeight: FontWeight.w600,
                      color: canSubmit ? context.colors.textBright : context.colors.textFaint,
                    ),
                  ),
                ),
              ),
            ],
          ),
          const Gap(4),
        ],
      ),
    );
  }
}

class _SelectionChip extends StatelessWidget {
  const _SelectionChip({required this.selected, required this.label, required this.onTap});

  final bool selected;
  final String label;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return Tappable(
      onTap: onTap,
      child: Container(
        padding: const Pad(horizontal: 12, vertical: 9),
        decoration: BoxDecoration(
          borderRadius: BorderRadius.circular(999),
          color: selected ? context.colors.accentBrandSubtle : context.colors.surfaceMuted,
          border: Border.all(color: selected ? context.colors.accentBrand : context.colors.borderDefault),
        ),
        child: Text(
          label,
          style: TextStyle(
            fontSize: 13,
            fontWeight: selected ? FontWeight.w600 : FontWeight.w500,
            color: selected ? context.colors.accentBrand : context.colors.textSubtle,
          ),
        ),
      ),
    );
  }
}

class _FeedbackTopic {
  const _FeedbackTopic({required this.value, required this.label});

  final String value;
  final String label;
}

class _FeedbackMood {
  const _FeedbackMood({required this.value, required this.icon});

  final String value;
  final IconData icon;
}

const feedbackTopics = [
  _FeedbackTopic(value: 'editor', label: '글쓰기/편집'),
  _FeedbackTopic(value: 'share', label: '발행/공유'),
  _FeedbackTopic(value: 'design', label: '테마/디자인'),
  _FeedbackTopic(value: 'billing', label: '구독/결제'),
  _FeedbackTopic(value: 'other', label: '기타'),
];

const feedbackMoods = [
  _FeedbackMood(value: 'angry', icon: LucideLightIcons.angry),
  _FeedbackMood(value: 'annoyed', icon: LucideLightIcons.annoyed),
  _FeedbackMood(value: 'good', icon: LucideLightIcons.smile),
  _FeedbackMood(value: 'great', icon: LucideLightIcons.laugh),
];
