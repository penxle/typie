import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:gap/gap.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/extensions/jiffy.dart';
import 'package:typie/extensions/num.dart';
import 'package:typie/graphql/__generated__/schema.schema.gql.dart';
import 'package:typie/graphql/widget.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/routers/app.gr.dart';
import 'package:typie/screens/current_plan/__generated__/current_plan_query.req.gql.dart';
import 'package:typie/widgets/heading.dart';
import 'package:typie/widgets/horizontal_divider.dart';
import 'package:typie/widgets/screen.dart';
import 'package:typie/widgets/tappable.dart';
import 'package:typie/widgets/vertical_divider.dart';

@RoutePage()
class CurrentPlanScreen extends StatelessWidget {
  const CurrentPlanScreen({super.key});

  @override
  Widget build(BuildContext context) {
    return Screen(
      heading: Heading(
        title: '이용권 정보',
        leadingWidget: HeadingLeading(
          icon: LucideLightIcons.chevron_left,
          onTap: () async {
            await context.router.maybePop();
          },
        ),
      ),
      expand: false,
      padding: const Pad(all: 20),
      child: GraphQLOperation(
        operation: GCurrentPlanScreen_QueryReq(),
        builder: (context, client, data) {
          return Container(
            decoration: BoxDecoration(
              border: Border.all(color: context.colors.borderStrong),
              borderRadius: BorderRadius.circular(8),
              color: context.colors.surfaceDefault,
            ),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                Padding(
                  padding: const Pad(all: 16),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.stretch,
                    children: [
                      Text('현재 이용권', style: TextStyle(fontSize: 14, color: context.colors.textFaint)),
                      const Gap(4),
                      Text(data.me!.subscription!.plan.name, style: const TextStyle(fontWeight: FontWeight.w600)),
                      const Gap(8),
                      Text(
                        '이용권 가격: ${data.me!.subscription!.plan.fee.comma}원',
                        style: TextStyle(fontSize: 14, color: context.colors.textFaint),
                      ),
                      Text(
                        '다음 결제일: ${data.me!.subscription!.expiresAt.toLocal().yyyyMMdd}',
                        style: TextStyle(fontSize: 14, color: context.colors.textFaint),
                      ),
                    ],
                  ),
                ),
                HorizontalDivider(color: context.colors.borderStrong),
                if (data.me!.subscription!.plan.availability == GPlanAvailability.IN_APP_PURCHASE)
                  SizedBox(
                    height: 48,
                    child: Row(
                      children: [
                        Expanded(
                          child: Tappable(
                            onTap: () async {
                              await context.router.push(const CancelPlanRoute());
                            },
                            child: const Center(
                              child: Text('해지하기', style: TextStyle(fontSize: 15, fontWeight: FontWeight.w500)),
                            ),
                          ),
                        ),
                        AppVerticalDivider(color: context.colors.borderStrong),
                        Expanded(
                          child: Tappable(
                            onTap: () async {
                              await context.router.push(const EnrollPlanRoute());
                            },
                            child: const Center(
                              child: Text('변경하기', style: TextStyle(fontSize: 15, fontWeight: FontWeight.w500)),
                            ),
                          ),
                        ),
                      ],
                    ),
                  )
                else if (data.me!.subscription!.plan.availability == GPlanAvailability.BILLING_KEY)
                  Padding(
                    padding: const Pad(all: 16),
                    child: Text(
                      '웹사이트에서 가입한 이용권이에요.\n정보 변경이 필요할 경우 웹사이트에서 진행해주세요.',
                      style: TextStyle(fontSize: 14, color: context.colors.textFaint),
                    ),
                  )
                else if (data.me!.subscription!.plan.availability == GPlanAvailability.MANUAL)
                  Padding(
                    padding: const Pad(all: 16),
                    child: Text(
                      '정보 변경을 할 수 없는 이용권이에요.\n정보 변경이 필요할 경우 고객센터에 문의해주세요.',
                      style: TextStyle(fontSize: 14, color: context.colors.textFaint),
                    ),
                  ),
              ],
            ),
          );
        },
      ),
    );
  }
}
