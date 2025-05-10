import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/graphql/error.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/logger.dart';
import 'package:typie/screens/login_with_email/__generated__/login_with_email_mutation.req.gql.dart';
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
      child: Center(
        child: Tappable(
          child: const Text('Login'),
          onTap: () async {
            try {
              await client.request(
                GLoginWithEmailScreen_LoginWithEmail_MutationReq(
                  (b) =>
                      b
                        ..vars.input.email = ''
                        ..vars.input.password = '',
                ),
              );
            } on TypieError catch (error) {
              logger.e('error', error: error);
            }
          },
        ),
      ),
    );
  }
}
