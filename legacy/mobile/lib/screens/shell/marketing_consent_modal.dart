import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:gap/gap.dart';
import 'package:jiffy/jiffy.dart';
import 'package:typie/context/modal.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/context/toast.dart';
import 'package:typie/extensions/jiffy.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/screens/shell/__generated__/update_marketing_consent_mutation.req.gql.dart';
import 'package:typie/widgets/tappable.dart';

class MarketingConsentModal extends StatelessWidget {
  const MarketingConsentModal({required this.client, super.key});

  final GraphQLClient client;

  @override
  Widget build(BuildContext context) {
    return Modal(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Center(
            child: SizedBox(
              height: 32,
              width: 32 + 4 * 22,
              child: Stack(
                children: [
                  for (var i = 0; i < 5; i++)
                    Positioned(
                      left: i * 22.0,
                      child: Container(
                        decoration: BoxDecoration(
                          color: context.colors.surfaceDark,
                          border: Border.all(color: context.colors.surfaceDefault, width: 2),
                          borderRadius: BorderRadius.circular(999),
                        ),
                        padding: const Pad(all: 6),
                        child: Icon(
                          [
                            LucideLightIcons.mail,
                            LucideLightIcons.bell,
                            LucideLightIcons.sparkles,
                            LucideLightIcons.zap,
                            LucideLightIcons.gift,
                          ][i],
                          size: 16,
                          color: context.colors.textBright,
                        ),
                      ),
                    ),
                ],
              ),
            ),
          ),
          const Gap(16),
          const Text(
            '타이피 소식 받아보기',
            textAlign: TextAlign.center,
            style: TextStyle(fontSize: 18, fontWeight: FontWeight.w700),
          ),
          const Gap(8),
          Text(
            '새 기능, 글쓰기 팁, 할인 혜택 등\n다양한 소식을 전해드려요.',
            textAlign: TextAlign.center,
            style: TextStyle(fontSize: 14, color: context.colors.textFaint),
          ),
          const Gap(24),
          Tappable(
            onTap: () => _handleConsent(context, true),
            child: Container(
              alignment: Alignment.center,
              decoration: BoxDecoration(color: context.colors.surfaceInverse, borderRadius: BorderRadius.circular(999)),
              padding: const Pad(vertical: 12),
              child: Text(
                '받을게요',
                style: TextStyle(fontSize: 16, fontWeight: FontWeight.w600, color: context.colors.textInverse),
              ),
            ),
          ),
          const Gap(8),
          Tappable(
            onTap: () => _handleConsent(context, false),
            child: Container(
              alignment: Alignment.center,
              decoration: BoxDecoration(color: context.colors.surfaceMuted, borderRadius: BorderRadius.circular(999)),
              padding: const Pad(vertical: 12),
              child: const Text('안 받을게요', style: TextStyle(fontSize: 16, fontWeight: FontWeight.w600)),
            ),
          ),
          const Gap(16),
          Text(
            '나중에 설정에서 변경할 수 있어요',
            textAlign: TextAlign.center,
            style: TextStyle(fontSize: 12, color: context.colors.textFaint),
          ),
        ],
      ),
    );
  }

  Future<void> _handleConsent(BuildContext context, bool consented) async {
    await client.request(
      GHomeScreen_UpdateMarketingConsent_MutationReq((b) => b..vars.input.marketingConsent = consented),
    );

    if (context.mounted) {
      await context.router.maybePop();
      if (context.mounted) {
        context.toast(ToastType.success, '${Jiffy.now().yyyyMMddKorean}에 마케팅 수신 ${consented ? '동의' : '거부'}처리됐어요.');
      }
    }
  }
}
