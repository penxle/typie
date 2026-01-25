import 'dart:async';

import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:built_value/json_object.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:gap/gap.dart';
import 'package:mixpanel_flutter/mixpanel_flutter.dart';
import 'package:typie/context/modal.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/providers/__generated__/record_survey_mutation.req.gql.dart';
import 'package:typie/providers/__generated__/survey_query.req.gql.dart';
import 'package:typie/routers/app.dart';
import 'package:typie/routers/app.gr.dart';
import 'package:typie/services/auth.dart';
import 'package:typie/widgets/tappable.dart';

class SurveyProvider extends HookWidget {
  const SurveyProvider({super.key});

  @override
  Widget build(BuildContext context) {
    final auth = useService<Auth>();
    final client = useService<GraphQLClient>();
    final router = useService<AppRouter>();
    final mixpanel = useService<Mixpanel>();
    final authState = useValueListenable(auth);

    useEffect(() {
      if (authState is Authenticated) {
        unawaited(
          client.request(GSurveyProvider_Me_QueryReq()).then((result) {
            final me = result.me;
            if (me != null && me.surveys.contains('trial_expired_modal')) {
              final navigatorContext = router.navigatorKey.currentContext;
              if (navigatorContext != null && navigatorContext.mounted) {
                unawaited(mixpanel.track('view_trial_expired_modal'));
                unawaited(
                  navigatorContext.showModal(
                    dismissible: false,
                    child: _TrialExpiredModal(client: client, mixpanel: mixpanel),
                  ),
                );
              }
            }
          }),
        );
      }

      return null;
    }, [authState]);

    return const SizedBox.shrink();
  }
}

class _TrialExpiredModal extends StatelessWidget {
  const _TrialExpiredModal({required this.client, required this.mixpanel});

  final GraphQLClient client;
  final Mixpanel mixpanel;

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
                            LucideLightIcons.crown,
                            LucideLightIcons.tag,
                            LucideLightIcons.star,
                            LucideLightIcons.key,
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
            '무료 체험이 종료됐어요',
            textAlign: TextAlign.center,
            style: TextStyle(fontSize: 18, fontWeight: FontWeight.w700),
          ),
          const Gap(8),
          Text(
            '무료 체험은 어떠셨나요?\n타이피의 모든 기능을 계속 이용하시려면\n플랜을 업그레이드해 주세요.',
            textAlign: TextAlign.center,
            style: TextStyle(fontSize: 14, color: context.colors.textFaint),
          ),
          const Gap(24),
          Tappable(
            onTap: () => _handleUpgrade(context),
            child: Container(
              alignment: Alignment.center,
              decoration: BoxDecoration(color: context.colors.surfaceInverse, borderRadius: BorderRadius.circular(999)),
              padding: const Pad(vertical: 12),
              child: Text(
                '지금 업그레이드',
                style: TextStyle(fontSize: 16, fontWeight: FontWeight.w600, color: context.colors.textInverse),
              ),
            ),
          ),
          const Gap(8),
          Tappable(
            onTap: () => _handleDismiss(context),
            child: Container(
              alignment: Alignment.center,
              decoration: BoxDecoration(color: context.colors.surfaceMuted, borderRadius: BorderRadius.circular(999)),
              padding: const Pad(vertical: 12),
              child: const Text('좀 더 둘러볼게요', style: TextStyle(fontSize: 16, fontWeight: FontWeight.w600)),
            ),
          ),
        ],
      ),
    );
  }

  Future<void> _handleDismiss(BuildContext context) async {
    await _markAsShown();
    unawaited(mixpanel.track('dismiss_trial_expired_modal'));
    if (context.mounted) {
      await context.router.maybePop();
    }
  }

  Future<void> _handleUpgrade(BuildContext context) async {
    await _markAsShown();
    unawaited(mixpanel.track('click_upgrade_from_trial_expired'));
    if (context.mounted) {
      await context.router.maybePop();
      if (context.mounted) {
        await context.router.push(const EnrollPlanRoute());
      }
    }
  }

  Future<void> _markAsShown() async {
    await client.request(
      GSurveyProvider_RecordSurvey_MutationReq(
        (b) => b
          ..vars.input.name = 'trial_expired_modal_shown'
          ..vars.input.value = JsonObject({}),
      ),
    );
  }
}
