import 'package:ferry/ferry.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:gap/gap.dart';
import 'package:typie/context/bottom_sheet.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/extensions/jiffy.dart';
import 'package:typie/extensions/num.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/native/editor_native.dart';
import 'package:typie/screens/native_editor/__generated__/native_editor_query.data.gql.dart';
import 'package:typie/screens/native_editor/__generated__/native_editor_query.req.gql.dart';

class InfoSheet extends HookWidget {
  const InfoSheet({required this.slug, required this.client, this.characterCounts, super.key});

  final String slug;
  final GraphQLClient client;
  final NativeEditorCharacterCounts? characterCounts;

  @override
  Widget build(BuildContext context) {
    final stream = useMemoized(
      () => client.raw
          .request(
            GNativeEditorScreen_QueryReq(
              (b) => b
                ..vars.slug = slug
                ..fetchPolicy = FetchPolicy.CacheOnly,
            ),
          )
          .where((response) => response.data != null)
          .map((response) => response.data!),
      [slug],
    );
    final snapshot = useStream(stream);

    final document = snapshot.data?.entity.node.when(document: (doc) => doc, orElse: () => null);
    if (document == null) {
      return const SizedBox.shrink();
    }

    final difference = document.characterCountChange.additions - document.characterCountChange.deletions;

    return AppBottomSheet(
      padding: const EdgeInsets.symmetric(horizontal: 20),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(
            '문서 정보',
            style: TextStyle(fontSize: 17, fontWeight: FontWeight.w700, color: context.colors.textSubtle),
          ),
          const Gap(12),
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Text('최초 생성 시각', style: TextStyle(fontSize: 14, color: context.colors.textFaint)),
              Text(
                '${document.createdAt.toLocal().yyyyMMdd} ${document.createdAt.toLocal().Hm}',
                style: TextStyle(fontSize: 14, fontWeight: FontWeight.w500, color: context.colors.textSubtle),
              ),
            ],
          ),
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Text('마지막 수정 시각', style: TextStyle(fontSize: 14, color: context.colors.textFaint)),
              Text(
                '${document.updatedAt.toLocal().yyyyMMdd} ${document.updatedAt.toLocal().Hm}',
                style: TextStyle(fontSize: 14, fontWeight: FontWeight.w500, color: context.colors.textSubtle),
              ),
            ],
          ),
          const Gap(32),
          Text(
            '본문 정보',
            style: TextStyle(fontSize: 17, fontWeight: FontWeight.w700, color: context.colors.textSubtle),
          ),
          const Gap(12),
          Row(
            spacing: 4,
            children: [
              Icon(LucideLightIcons.type_, size: 15, color: context.colors.textSubtle),
              Text(
                '글자 수',
                style: TextStyle(fontSize: 15, fontWeight: FontWeight.w500, color: context.colors.textSubtle),
              ),
            ],
          ),
          const Gap(8),
          _CharacterCountRow(
            label: '공백 포함',
            docCount: characterCounts?.docWithWhitespace ?? 0,
            selectionCount: characterCounts?.selectionWithWhitespace ?? 0,
          ),
          _CharacterCountRow(
            label: '공백 미포함',
            docCount: characterCounts?.docWithoutWhitespace ?? 0,
            selectionCount: characterCounts?.selectionWithoutWhitespace ?? 0,
          ),
          _CharacterCountRow(
            label: '공백/부호 미포함',
            docCount: characterCounts?.docWithoutWhitespaceAndPunctuation ?? 0,
            selectionCount: characterCounts?.selectionWithoutWhitespaceAndPunctuation ?? 0,
          ),
          const Gap(12),
          Row(
            spacing: 4,
            children: [
              Icon(LucideLightIcons.goal, size: 15, color: context.colors.textSubtle),
              Text(
                '오늘의 기록',
                style: TextStyle(fontSize: 15, fontWeight: FontWeight.w500, color: context.colors.textSubtle),
              ),
            ],
          ),
          const Gap(8),
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Expanded(
                child: Text('변화량', style: TextStyle(fontSize: 14, color: context.colors.textFaint)),
              ),
              if (difference == 0)
                Text(
                  '없음',
                  style: TextStyle(fontSize: 14, fontWeight: FontWeight.w500, color: context.colors.textSubtle),
                )
              else ...[
                Icon(difference >= 0 ? LucideLightIcons.trending_up : LucideLightIcons.trending_down, size: 14),
                const Gap(4),
                Text(
                  '${difference >= 0 ? '+' : '-'}${difference.abs().comma}자',
                  style: TextStyle(fontSize: 14, fontWeight: FontWeight.w500, color: context.colors.textSubtle),
                ),
              ],
            ],
          ),
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Text('입력한 글자', style: TextStyle(fontSize: 14, color: context.colors.textFaint)),
              Text(
                '${document.characterCountChange.additions.comma}자',
                style: TextStyle(fontSize: 14, fontWeight: FontWeight.w500, color: context.colors.textSubtle),
              ),
            ],
          ),
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Text('지운 글자', style: TextStyle(fontSize: 14, color: context.colors.textFaint)),
              Text(
                '${document.characterCountChange.deletions.comma}자',
                style: TextStyle(fontSize: 14, fontWeight: FontWeight.w500, color: context.colors.textSubtle),
              ),
            ],
          ),
        ],
      ),
    );
  }
}

class _CharacterCountRow extends StatelessWidget {
  const _CharacterCountRow({required this.label, required this.docCount, required this.selectionCount});

  final String label;
  final int docCount;
  final int selectionCount;

  @override
  Widget build(BuildContext context) {
    final hasSelection = selectionCount > 0;

    return Row(
      mainAxisAlignment: MainAxisAlignment.spaceBetween,
      children: [
        Text(label, style: TextStyle(fontSize: 14, color: context.colors.textFaint)),
        Text(
          hasSelection ? '${selectionCount.comma}자 / ${docCount.comma}자' : '${docCount.comma}자',
          style: TextStyle(fontSize: 14, fontWeight: FontWeight.w500, color: context.colors.textSubtle),
        ),
      ],
    );
  }
}
