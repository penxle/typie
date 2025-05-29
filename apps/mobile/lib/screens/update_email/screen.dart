import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:gap/gap.dart';
import 'package:luthor/luthor.dart';
import 'package:typie/context/modal.dart';
import 'package:typie/context/toast.dart';
import 'package:typie/graphql/error.dart';
import 'package:typie/graphql/widget.dart';
import 'package:typie/screens/update_email/__generated__/screen.req.gql.dart';
import 'package:typie/styles/colors.dart';
import 'package:typie/widgets/forms/form.dart';
import 'package:typie/widgets/forms/text_field.dart';
import 'package:typie/widgets/heading.dart';
import 'package:typie/widgets/screen.dart';
import 'package:typie/widgets/tappable.dart';

@RoutePage()
class UpdateEmailScreen extends HookWidget {
  const UpdateEmailScreen({super.key});

  @override
  Widget build(BuildContext context) {
    return Screen(
      heading: const Heading(title: '이메일 변경'),
      safeArea: false,
      resizeToAvoidBottomInset: true,
      padding: const Pad(top: 20),
      child: GraphQLOperation(
        operation: GUpdateEmailScreen_QueryReq(),
        builder: (context, client, data) {
          return HookForm(
            schema: l.schema({
              'email': l.string().email(message: '유효한 이메일 주소를 입력해주세요.')..required(message: '이메일 주소를 입력해주세요.'),
            }),
            onSubmit: (form) async {
              try {
                await client.request(
                  GUpdateEmailScreen_SendEmailUpdateEmail_MutationReq(
                    (b) => b..vars.input.email = form.data['email'] as String,
                  ),
                );

                if (context.mounted) {
                  await context.showModal(
                    child: Modal(
                      child: Column(
                        mainAxisSize: MainAxisSize.min,
                        crossAxisAlignment: CrossAxisAlignment.stretch,
                        children: [
                          const Text('변경할 이메일 주소로 인증 메일을 발송했어요. 메일함을 확인해주세요.'),
                          const Gap(24),
                          Tappable(
                            onTap: () async {
                              await context.router.root.maybePop();
                              if (context.mounted) {
                                await context.router.maybePop();
                              }
                            },
                            child: Container(
                              decoration: BoxDecoration(
                                color: AppColors.gray_100,
                                borderRadius: BorderRadius.circular(999),
                              ),
                              padding: const Pad(vertical: 12),
                              child: const Text(
                                '확인',
                                textAlign: TextAlign.center,
                                style: TextStyle(fontSize: 16, fontWeight: FontWeight.w600),
                              ),
                            ),
                          ),
                        ],
                      ),
                    ),
                  );
                }
              } on TypieError catch (e) {
                if (context.mounted) {
                  context.toast(ToastType.error, switch (e.code) {
                    'user_email_exists' => '이미 사용중인 이메일이에요.',
                    _ => '오류가 발생했어요. 잠시 후 다시 시도해주세요.',
                  }, bottom: 64);
                }
              }
            },
            builder: (context, form) {
              return Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                spacing: 16,
                children: [
                  Padding(
                    padding: const Pad(horizontal: 20),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.stretch,
                      spacing: 2,
                      children: [
                        const Text('현재 이메일 주소', style: TextStyle(fontSize: 14, fontWeight: FontWeight.w500)),
                        Text(
                          data.me!.email,
                          style: const TextStyle(fontSize: 16, fontWeight: FontWeight.w500, color: AppColors.gray_600),
                        ),
                      ],
                    ),
                  ),
                  const Padding(
                    padding: Pad(horizontal: 20),
                    child: HookFormTextField(
                      name: 'email',
                      label: '변경할 이메일 주소',
                      placeholder: '변경할 이메일 주소',
                      autofocus: true,
                    ),
                  ),
                  const Spacer(),
                  Tappable(
                    child: Container(
                      alignment: Alignment.center,
                      decoration: const BoxDecoration(color: AppColors.gray_950),
                      padding: Pad(vertical: 16, bottom: MediaQuery.paddingOf(context).bottom),
                      child: const Text(
                        '변경',
                        style: TextStyle(fontSize: 16, fontWeight: FontWeight.w700, color: AppColors.white),
                      ),
                    ),
                    onTap: () async {
                      await form.submit();
                    },
                  ),
                ],
              );
            },
          );
        },
      ),
    );
  }
}
