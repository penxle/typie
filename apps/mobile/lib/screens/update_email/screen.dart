import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:gap/gap.dart';
import 'package:luthor/luthor.dart';
import 'package:mixpanel_flutter/mixpanel_flutter.dart';
import 'package:typie/context/modal.dart';
import 'package:typie/context/toast.dart';
import 'package:typie/graphql/error.dart';
import 'package:typie/graphql/widget.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/screens/update_email/__generated__/screen_query.req.gql.dart';
import 'package:typie/screens/update_email/__generated__/send_email_update_email_mutation.req.gql.dart';
import 'package:typie/widgets/forms/form.dart';
import 'package:typie/widgets/forms/text_field.dart';
import 'package:typie/widgets/heading.dart';
import 'package:typie/widgets/screen.dart';

@RoutePage()
class UpdateEmailScreen extends HookWidget {
  const UpdateEmailScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final form = useHookForm();
    final mixpanel = useService<Mixpanel>();

    return Screen(
      heading: const Heading(title: '이메일 변경'),
      resizeToAvoidBottomInset: true,
      padding: const Pad(top: 20),
      bottomAction: BottomAction(
        text: '변경',
        onTap: () async {
          await form.submit();
        },
      ),
      child: GraphQLOperation(
        operation: GUpdateEmailScreen_QueryReq(),
        builder: (context, client, data) {
          return HookForm(
            form: form,
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

                await mixpanel.track('send_email_update_email');

                if (context.mounted) {
                  await context.showModal(
                    child: const AlertModal(title: '이메일 인증', message: '변경할 이메일 주소로 인증 메일을 발송했어요. 메일함을 확인해주세요.'),
                  );
                }

                if (context.mounted) {
                  await context.router.maybePop();
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
                children: [
                  Padding(
                    padding: const Pad(horizontal: 20),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.stretch,
                      spacing: 4,
                      children: [
                        const Text('현재 이메일 주소', style: TextStyle(fontSize: 14, fontWeight: FontWeight.w500)),
                        Text(data.me!.email, style: const TextStyle(fontSize: 16, fontWeight: FontWeight.w500)),
                      ],
                    ),
                  ),
                  const Gap(20),
                  const Padding(
                    padding: Pad(horizontal: 20),
                    child: HookFormTextField(
                      name: 'email',
                      label: '변경할 이메일 주소',
                      placeholder: 'me@example.com',
                      keyboardType: TextInputType.emailAddress,
                      autofillHints: [AutofillHints.email],
                      autofocus: true,
                    ),
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
