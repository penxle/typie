import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:luthor/luthor.dart';
import 'package:typie/context/toast.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/graphql/error.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/screens/login_with_email/__generated__/login_with_email_mutation.req.gql.dart';
import 'package:typie/widgets/forms/form.dart';
import 'package:typie/widgets/forms/text_field.dart';
import 'package:typie/widgets/heading.dart';
import 'package:typie/widgets/screen.dart';
import 'package:typie/widgets/tappable.dart';

@RoutePage()
class LoginWithEmailScreen extends HookWidget {
  const LoginWithEmailScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final client = useService<GraphQLClient>();

    return Screen(
      appBar: const EmptyHeading(),
      padding: const Pad(horizontal: 20),
      child: HookForm(
        schema: l.schema({
          'email': l
              .string()
              .min(1, message: '이메일 주소를 입력해주세요.')
              .email(message: '올바른 이메일 주소를 입력해주세요.')
              .required(message: '이메일 주소를 입력해주세요.'),
          'password': l.string().min(1, message: '비밀번호를 입력해주세요.').required(message: '비밀번호를 입력해주세요.'),
        }),
        onSubmit: (form) async {
          try {
            await client.request(
              GLoginWithEmailScreen_LoginWithEmail_MutationReq((b) {
                b.vars.input.email = form.data['email'] as String;
                b.vars.input.password = form.data['password'] as String;
              }),
            );
          } on TypieError catch (e) {
            if (e.code == 'invalid_credentials') {
              if (context.mounted) {
                context.toast.show(ToastType.error, '이메일 주소 또는 비밀번호가 올바르지 않습니다.');
              }
            }
          }
        },
        builder: (context, form) {
          return Column(
            mainAxisAlignment: MainAxisAlignment.center,
            spacing: 16,
            children: [
              const FormTextField(
                name: 'email',
                labelText: 'Email',
                keyboardType: TextInputType.emailAddress,
                textInputAction: TextInputAction.next,
              ),
              const FormTextField(
                name: 'password',
                labelText: 'Password',
                obscureText: true,
                keyboardType: TextInputType.visiblePassword,
                textInputAction: TextInputAction.done,
              ),
              Tappable(
                child: const Text('Login'),
                onTap: () {
                  form.submit();
                },
              ),
            ],
          );
        },
      ),
    );
  }
}
