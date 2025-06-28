import 'dart:async';

import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:gap/gap.dart';
import 'package:mixpanel_flutter/mixpanel_flutter.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/context/toast.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/screens/delete_user/__generated__/delete_user_mutation.req.gql.dart';
import 'package:typie/services/auth.dart';
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
    final mixpanel = useService<Mixpanel>();

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
          Text(
            '탈퇴 전 아래 유의사항을 확인해주세요.',
            textAlign: TextAlign.center,
            style: TextStyle(fontSize: 14, color: context.colors.textFaint),
          ),
          const Gap(24),
          Container(
            decoration: BoxDecoration(
              border: Border.all(color: context.colors.borderStrong),
              borderRadius: BorderRadius.circular(8),
              color: context.colors.surfaceDefault,
            ),
            padding: const Pad(all: 16),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                const Text('유의사항', style: TextStyle(fontSize: 15, fontWeight: FontWeight.w500)),
                const Gap(4),
                Text(
                  '- 작성한 모든 글과 데이터는 탈퇴와 함께 삭제되며 재가입시에도 복구할 수 없어요.',
                  style: TextStyle(fontSize: 14, color: context.colors.textSubtle),
                ),
                Text(
                  '- 이용중인 사이트 주소는 다시 이용할 수 없어요. 사이트 주소를 다시 사용할 계획이라면, 탈퇴 전 기존 주소를 변경해주세요.',
                  style: TextStyle(fontSize: 14, color: context.colors.textSubtle),
                ),
                Text(
                  '- 남은 이용권 기간은 탈퇴와 함께 소멸되며, 환불은 별도로 제공되지 않아요.',
                  style: TextStyle(fontSize: 14, color: context.colors.textSubtle),
                ),
                Text(
                  '- 스토어에서 이용권을 구매했을 경우, 구독 취소 처리는 스토어 규정상 스토어 내 설정에서 직접 진행해야 해요.',
                  style: TextStyle(fontSize: 14, color: context.colors.textSubtle),
                ),
              ],
            ),
          ),
          const Gap(12),
          Tappable(
            onTap: () {
              isChecked.value = !isChecked.value;
            },
            child: Row(
              mainAxisAlignment: MainAxisAlignment.center,
              spacing: 8,
              children: [
                Container(
                  decoration: BoxDecoration(
                    border: Border.all(color: context.colors.borderStrong),
                    borderRadius: const BorderRadius.all(Radius.circular(2)),
                    color: isChecked.value ? context.colors.textDefault : context.colors.surfaceDefault,
                  ),
                  padding: const Pad(all: 1),
                  child: Icon(LucideLightIcons.check, size: 12, color: context.colors.surfaceDefault),
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

              unawaited(mixpanel.track('delete_user'));

              await auth.clearTokens();
            },
            child: Container(
              alignment: Alignment.center,
              decoration: BoxDecoration(
                border: Border.all(color: context.colors.borderStrong),
                borderRadius: BorderRadius.circular(8),
                color: context.colors.accentDanger,
              ),
              padding: const Pad(vertical: 12),
              child: Text(
                '탈퇴하기',
                style: TextStyle(fontSize: 16, fontWeight: FontWeight.w500, color: context.colors.textBright),
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
                border: Border.all(color: context.colors.borderStrong),
                borderRadius: BorderRadius.circular(8),
                color: context.colors.surfaceDefault,
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
