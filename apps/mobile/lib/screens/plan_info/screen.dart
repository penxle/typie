import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:typie/extensions/num.dart';
import 'package:typie/graphql/widget.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/routers/app.gr.dart';
import 'package:typie/screens/plan_info/__generated__/screen.req.gql.dart';
import 'package:typie/styles/colors.dart';
import 'package:typie/widgets/heading.dart';
import 'package:typie/widgets/screen.dart';
import 'package:typie/widgets/tappable.dart';

@RoutePage()
class PlanInfoScreen extends StatelessWidget {
  const PlanInfoScreen({super.key});

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
      child: GraphQLOperation(
        operation: GPlanInfoScreen_QueryReq(),
        builder: (context, client, data) {
          return Container(
            padding: const Pad(all: 20),
            child: Container(
              decoration: BoxDecoration(
                border: Border.all(color: AppColors.gray_950),
                borderRadius: BorderRadius.circular(8),
                color: AppColors.white,
              ),
              padding: const Pad(horizontal: 16, vertical: 20),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                mainAxisSize: MainAxisSize.min,
                spacing: 8,
                children: [
                  const Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    spacing: 2,
                    children: [
                      Text('현재 이용권', style: TextStyle(fontSize: 14, color: AppColors.gray_600)),
                      Text('타이피 FULL ACCESS', style: TextStyle(fontWeight: FontWeight.w600)),
                    ],
                  ),
                  Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    spacing: 2,
                    children: [
                      Text(
                        '이용기간: ${data.me!.plan!.createdAt.format(pattern: 'yyyy.MM.dd')} - ${data.me!.plan!.expiresAt.format(pattern: 'yyyy.MM.dd')}',
                        style: const TextStyle(fontSize: 12, color: AppColors.gray_500),
                      ),
                      Text(
                        '결제 예정일: ${data.me!.plan!.nextInvoice!.billingAt.format(pattern: 'yyyy.MM.dd')}',
                        style: const TextStyle(fontSize: 12, color: AppColors.gray_500),
                      ),
                      Text(
                        '결제 예정 금액: ${data.me!.plan!.nextInvoice!.amount.comma}원',
                        style: const TextStyle(fontSize: 12, color: AppColors.gray_500),
                      ),
                    ],
                  ),
                  Row(
                    spacing: 8,
                    children: [
                      Expanded(
                        child: Tappable(
                          onTap: () async {
                            await context.router.push(const CancelPlanRoute());
                          },
                          child: Container(
                            decoration: BoxDecoration(
                              border: Border.all(color: AppColors.gray_950),
                              borderRadius: BorderRadius.circular(8),
                              color: AppColors.gray_100,
                            ),
                            padding: const Pad(horizontal: 10, vertical: 12),
                            child: const Text(
                              '이용권 해지',
                              textAlign: TextAlign.center,
                              style: TextStyle(fontWeight: FontWeight.w500, fontSize: 15),
                            ),
                          ),
                        ),
                      ),
                      Expanded(
                        child: Tappable(
                          onTap: () async {
                            await context.router.push(const PurchasePlanRoute());
                          },
                          child: Container(
                            decoration: BoxDecoration(
                              border: Border.all(color: AppColors.gray_950),
                              borderRadius: BorderRadius.circular(8),
                              color: AppColors.white,
                            ),
                            padding: const Pad(horizontal: 10, vertical: 12),
                            child: const Text(
                              '이용권 변경',
                              textAlign: TextAlign.center,
                              style: TextStyle(fontWeight: FontWeight.w500, fontSize: 15),
                            ),
                          ),
                        ),
                      ),
                    ],
                  ),
                ],
              ),
            ),
          );
        },
      ),
    );
  }
}
