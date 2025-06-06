import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:gap/gap.dart';
import 'package:typie/styles/colors.dart';
import 'package:typie/widgets/tappable.dart';

class AppErrorWidget extends StatelessWidget {
  const AppErrorWidget({super.key});

  @override
  Widget build(BuildContext context) {
    return Material(
      color: AppColors.white,
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          const Text('앗! 문제가 발생했어요', style: TextStyle(fontSize: 16)),
          const Text('잠시 후 다시 시도해주세요.', style: TextStyle(fontSize: 15, color: AppColors.gray_500)),
          if (context.router.canPop()) ...[
            const Gap(16),
            Tappable(
              onTap: () async {
                await context.router.maybePop();
              },
              child: Container(
                decoration: BoxDecoration(
                  border: Border.all(color: AppColors.gray_950),
                  borderRadius: BorderRadius.circular(8),
                ),
                padding: const Pad(horizontal: 16, vertical: 8),
                child: const Text('뒤로 가기', style: TextStyle(fontSize: 15)),
              ),
            ),
          ],
        ],
      ),
    );
  }
}
