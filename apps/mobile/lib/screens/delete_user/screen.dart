import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:gap/gap.dart';
import 'package:typie/context/toast.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/screens/delete_user/__generated__/screen.req.gql.dart';
import 'package:typie/services/auth.dart';
import 'package:typie/styles/colors.dart';
import 'package:typie/widgets/heading.dart';
import 'package:typie/widgets/screen.dart';
import 'package:typie/widgets/tappable.dart';

@RoutePage()
class DeleteUserScreen extends HookWidget {
  const DeleteUserScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final auth = useService<Auth>();
    final client = useService<GraphQLClient>();

    final isChecked = useState(false);

    return Screen(
      heading: const Heading(title: '회원 탈퇴'),
      padding: const Pad(horizontal: 20, top: 40),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          const Text(
            '정말 탈퇴하시겠어요?',
            textAlign: TextAlign.center,
            style: TextStyle(fontSize: 18, fontWeight: FontWeight.w600),
          ),
          const Gap(4),
          const Text(
            '지금 탈퇴하시면, 아래 내용이 즉시 적용돼요.',
            textAlign: TextAlign.center,
            style: TextStyle(fontSize: 14, color: AppColors.gray_500),
          ),
          const Gap(24),
          Container(
            decoration: BoxDecoration(
              border: Border.all(color: AppColors.gray_950),
              borderRadius: BorderRadius.circular(8),
              color: AppColors.white,
            ),
            padding: const Pad(all: 16),
            child: const Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text('유의사항', style: TextStyle(fontSize: 15, fontWeight: FontWeight.w500)),
                Gap(4),
                Text('- 작성한 모든 글과 데이터는 삭제되며 복구할 수 없어요.', style: TextStyle(fontSize: 14, color: AppColors.gray_700)),
                Text(
                  '- 탈퇴 시 회원 정보는 모두 삭제되며, 재가입하더라도 삭제된 데이터는 복구되지 않아요.',
                  style: TextStyle(fontSize: 14, color: AppColors.gray_700),
                ),
                Text('- 프리미엄 혜택은 더 이상 이용할 수 없어요.', style: TextStyle(fontSize: 14, color: AppColors.gray_700)),
                Text('- 남은 기간의 환불은 제공되지 않아요.', style: TextStyle(fontSize: 14, color: AppColors.gray_700)),
              ],
            ),
          ),
          const Gap(8),
          Tappable(
            onTap: () {
              isChecked.value = !isChecked.value;
            },
            child: Row(
              spacing: 4,
              children: [
                Container(
                  decoration: BoxDecoration(
                    border: Border.all(color: AppColors.gray_950),
                    borderRadius: const BorderRadius.all(Radius.circular(2)),
                    color: isChecked.value ? AppColors.gray_950 : AppColors.white,
                  ),
                  padding: const Pad(all: 1),
                  child: const Icon(LucideLightIcons.check, size: 12, color: AppColors.white),
                ),
                const Text('위 유의사항을 모두 확인했어요', style: TextStyle(fontSize: 14)),
              ],
            ),
          ),
          const Gap(24),
          Tappable(
            onTap: () async {
              if (!isChecked.value) {
                context.toast(ToastType.error, '유의사항을 모두 확인해주세요.');
                return;
              }

              await client.request(GDeleteUserScreen_DeleteUser_MutationReq());
              await auth.clearTokens();
            },
            child: Container(
              alignment: Alignment.center,
              decoration: BoxDecoration(
                border: Border.all(color: AppColors.gray_950),
                borderRadius: BorderRadius.circular(8),
                color: AppColors.red_500,
              ),
              padding: const Pad(vertical: 12),
              child: const Text(
                '탈퇴하기',
                style: TextStyle(fontSize: 16, fontWeight: FontWeight.w500, color: AppColors.white),
              ),
            ),
          ),
          const Gap(8),
          Tappable(
            onTap: () async {
              await context.router.maybePop();
            },
            child: Container(
              alignment: Alignment.center,
              decoration: BoxDecoration(
                border: Border.all(color: AppColors.gray_950),
                borderRadius: BorderRadius.circular(8),
                color: AppColors.white,
              ),
              padding: const Pad(vertical: 12),
              child: const Text('타이피 계속 이용하기', style: TextStyle(fontSize: 16)),
            ),
          ),
        ],
      ),
    );
  }
}
