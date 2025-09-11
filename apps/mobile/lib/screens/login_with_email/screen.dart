import 'dart:async';

import 'package:appsflyer_sdk/appsflyer_sdk.dart';
import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:gap/gap.dart';
import 'package:luthor/luthor.dart';
import 'package:mixpanel_flutter/mixpanel_flutter.dart';
import 'package:typie/context/loader.dart';
import 'package:typie/context/toast.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/graphql/error.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/screens/login_with_email/__generated__/login_with_email_mutation.req.gql.dart';
import 'package:typie/widgets/forms/form.dart';
import 'package:typie/widgets/forms/text_field.dart';
import 'package:typie/widgets/heading.dart';
import 'package:typie/widgets/screen.dart';

@RoutePage()
class LoginWithEmailScreen extends HookWidget {
  const LoginWithEmailScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final client = useService<GraphQLClient>();
    final mixpanel = useService<Mixpanel>();
    final appsflyer = useService<AppsflyerSdk>();
    final form = useHookForm();

    return Screen(
      heading: const Heading(title: '이메일로 로그인'),
      resizeToAvoidBottomInset: true,
      bottomAction: BottomAction(
        text: '로그인',
        onTap: () async {
          await form.submit();
        },
      ),
      child: HookForm(
        form: form,
        schema: l.schema({
          'email': l
              .string()
              .min(1, message: '이메일을 입력해주세요.')
              .email(message: '올바른 이메일 형식을 입력해주세요.')
              .required(message: '이메일을 입력해주세요.'),
          'password': l.string().min(1, message: '비밀번호를 입력해주세요.').required(message: '비밀번호를 입력해주세요.'),
        }),
        onSubmit: (form) async {
          await context.runWithLoader(() async {
            try {
              unawaited(mixpanel.track('login_with_email'));
              unawaited(appsflyer.logEvent('sign_in', null));

              await client.request(
                GLoginWithEmailScreen_LoginWithEmail_MutationReq(
                  (b) => b
                    ..vars.input.email = form.data['email'] as String
                    ..vars.input.password = form.data['password'] as String,
                ),
              );
            } on TypieError catch (e) {
              if (context.mounted) {
                context.toast(ToastType.error, switch (e.code) {
                  'invalid_credentials' => '이메일 또는 비밀번호가 올바르지 않아요.',
                  'password_not_set' => '비밀번호가 설정되지 않았어요.',
                  _ => '오류가 발생했어요. 잠시 후 다시 시도해주세요.',
                }, bottom: 64);
              }
            }
          });
        },
        builder: (context, form) {
          return const AutofillGroup(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                Gap(24),
                Padding(
                  padding: Pad(horizontal: 20),
                  child: HookFormTextField(
                    name: 'email',
                    label: '이메일',
                    placeholder: 'me@example.com',
                    keyboardType: TextInputType.emailAddress,
                    textInputAction: TextInputAction.next,
                    autofillHints: [AutofillHints.email],
                    autofocus: true,
                  ),
                ),
                Gap(16),
                Padding(
                  padding: Pad(horizontal: 20),
                  child: HookFormTextField(
                    name: 'password',
                    label: '비밀번호',
                    placeholder: '********',
                    obscureText: true,
                    keyboardType: TextInputType.visiblePassword,
                    autofillHints: [AutofillHints.password],
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
