import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:gap/gap.dart';
import 'package:typie/extensions/jiffy.dart';
import 'package:typie/extensions/num.dart';
import 'package:typie/graphql/widget.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/routers/app.gr.dart';
import 'package:typie/screens/current_plan/__generated__/screen.req.gql.dart';
import 'package:typie/styles/colors.dart';
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
              border: Border.all(color: AppColors.gray_950),
              borderRadius: BorderRadius.circular(8),
              color: AppColors.white,
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
                      const Text('현재 이용권', style: TextStyle(fontSize: 14, color: AppColors.gray_500)),
                      const Gap(4),
                      Text(data.me!.subscription!.plan.name, style: const TextStyle(fontWeight: FontWeight.w600)),
                      const Gap(8),
                      Text(
                        '이용기간: ${data.me!.subscription!.startsAt.yyyyMMdd} - ${data.me!.subscription!.expiresAt.yyyyMMdd}',
                        style: const TextStyle(fontSize: 12, color: AppColors.gray_500),
                      ),
                      Text(
                        '다음 결제 예정 금액: ${data.me!.subscription!.plan.fee.comma}원',
                        style: const TextStyle(fontSize: 14, color: AppColors.gray_500),
                      ),
                    ],
                  ),
                ),
                const HorizontalDivider(color: AppColors.gray_950),
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
                      const AppVerticalDivider(color: AppColors.gray_950),
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
                ),
              ],
            ),
          );
        },
      ),
    );
  }
}
