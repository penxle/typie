import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:luthor/luthor.dart';
import 'package:mixpanel_flutter/mixpanel_flutter.dart';
import 'package:typie/context/toast.dart';
import 'package:typie/graphql/error.dart';
import 'package:typie/graphql/widget.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/screens/update_password/__generated__/screen_query.req.gql.dart';
import 'package:typie/screens/update_password/__generated__/update_password_mutation.req.gql.dart';
import 'package:typie/widgets/forms/form.dart';
import 'package:typie/widgets/forms/text_field.dart';
import 'package:typie/widgets/heading.dart';
import 'package:typie/widgets/screen.dart';

@RoutePage()
class UpdatePasswordScreen extends HookWidget {
  const UpdatePasswordScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final form = useHookForm();
    final mixpanel = useService<Mixpanel>();

    return GraphQLOperation(
      operation: GUpdatePasswordScreen_QueryReq(),
      builder: (context, client, data) {
        return Screen(
          heading: const Heading(title: '비밀번호 변경'),
          resizeToAvoidBottomInset: true,
          padding: const Pad(top: 20),
          bottomAction: BottomAction(
            text: (data.me?.hasPassword ?? false) ? '변경' : '설정',
            onTap: () async {
              await form.submit();
            },
          ),
          child: HookForm(
            form: form,
            schema: l.schema({
              'currentPassword': (data.me?.hasPassword ?? false)
                  ? l.string().min(1, message: '현재 비밀번호를 입력해주세요.').required(message: '현재 비밀번호를 입력해주세요.')
                  : l.string(),
              'newPassword': l.string().min(1, message: '새 비밀번호를 입력해주세요.').required(message: '새 비밀번호를 입력해주세요.'),
              'confirmPassword': l.string().min(1, message: '비밀번호 확인을 입력해주세요.').required(message: '비밀번호 확인을 입력해주세요.'),
            }),
            onSubmit: (form) async {
              final hasPassword = data.me?.hasPassword ?? false;
              final currentPassword = form.data['currentPassword'] as String?;
              final newPassword = form.data['newPassword'] as String;
              final confirmPassword = form.data['confirmPassword'] as String;

              if (newPassword != confirmPassword) {
                form.setError('confirmPassword', '비밀번호가 일치하지 않아요.');
                return;
              }

              try {
                await client.request(
                  GUpdatePasswordScreen_UpdatePassword_MutationReq(
                    (b) => b
                      ..vars.input.currentPassword = hasPassword ? currentPassword : null
                      ..vars.input.newPassword = newPassword,
                  ),
                );

                await mixpanel.track('update_password', properties: {'hadPassword': hasPassword});

                if (context.mounted) {
                  context.toast(ToastType.success, '비밀번호가 변경되었어요.');
                  await context.router.maybePop();
                }
              } on TypieError catch (e) {
                if (context.mounted) {
                  switch (e.code) {
                    case 'invalid_password':
                      form.setError('currentPassword', '비밀번호가 일치하지 않습니다.');
                    case 'current_password_required':
                      form.setError('currentPassword', '현재 비밀번호를 입력해주세요.');
                    default:
                      context.toast(ToastType.error, '오류가 발생했어요. 잠시 후 다시 시도해주세요.', bottom: 64);
                  }
                }
              }
            },
            builder: (context, form) {
              final hasPassword = data.me?.hasPassword ?? false;

              return Column(
                spacing: 20,
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  if (hasPassword)
                    const Padding(
                      padding: Pad(horizontal: 20),
                      child: HookFormTextField(
                        name: 'currentPassword',
                        label: '현재 비밀번호',
                        placeholder: '현재 비밀번호를 입력하세요',
                        obscureText: true,
                        autofillHints: [AutofillHints.password],
                        autofocus: true,
                      ),
                    ),
                  Padding(
                    padding: const Pad(horizontal: 20),
                    child: HookFormTextField(
                      name: 'newPassword',
                      label: '새 비밀번호',
                      placeholder: '********',
                      obscureText: true,
                      autofillHints: const [AutofillHints.password],
                      autofocus: !hasPassword,
                    ),
                  ),
                  const Padding(
                    padding: Pad(horizontal: 20),
                    child: HookFormTextField(
                      name: 'confirmPassword',
                      label: '새 비밀번호 확인',
                      placeholder: '********',
                      obscureText: true,
                      autofillHints: [AutofillHints.password],
                    ),
                  ),
                ],
              );
            },
          ),
        );
      },
    );
  }
}
