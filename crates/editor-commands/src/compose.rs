use crate::CommandResult;
use editor_state::State;
use editor_transaction::Transaction;

pub fn first(
    tr: &mut Transaction,
    commands: &[&dyn Fn(&mut Transaction) -> CommandResult],
) -> CommandResult {
    let sp = tr.savepoint();
    for cmd in commands {
        match cmd(tr)? {
            true => return Ok(true),
            false => tr.rollback(sp.clone()),
        }
    }
    Ok(false)
}

pub fn chain(
    tr: &mut Transaction,
    commands: &[&dyn Fn(&mut Transaction) -> CommandResult],
) -> CommandResult {
    let sp = tr.savepoint();
    for cmd in commands {
        if !cmd(tr)? {
            tr.rollback(sp);
            return Ok(false);
        }
    }
    Ok(true)
}

pub fn can(state: &State, cmd: &dyn Fn(&mut Transaction) -> CommandResult) -> CommandResult {
    let mut tr = Transaction::new(state);
    cmd(&mut tr)
}

pub fn optional(
    tr: &mut Transaction,
    cmd: &dyn Fn(&mut Transaction) -> CommandResult,
) -> CommandResult {
    match cmd(tr) {
        Ok(_) => Ok(true),
        Err(e) => Err(e),
    }
}

#[macro_export]
macro_rules! first {
    (@arm $($path:ident)::+ ( $($args:expr),* $(,)? )) => {
        &|tr| $($path)::+(tr $(, $args)*)
    };
    (@arm $cmd:expr) => {
        &$cmd
    };
    (@collect $tr:expr, [ $($arms:expr),* ] $($path:ident)::+ ( $($args:expr),* $(,)? ) , $($rest:tt)+) => {
        $crate::first!(@collect $tr, [ $($arms,)* $crate::first!(@arm $($path)::+($($args),*)) ] $($rest)+)
    };
    (@collect $tr:expr, [ $($arms:expr),* ] $($path:ident)::+ ( $($args:expr),* $(,)? ) $(,)?) => {
        $crate::first($tr, &[ $($arms,)* $crate::first!(@arm $($path)::+($($args),*)) ])
    };
    (@collect $tr:expr, [ $($arms:expr),* ] $cmd:expr , $($rest:tt)+) => {
        $crate::first!(@collect $tr, [ $($arms,)* $crate::first!(@arm $cmd) ] $($rest)+)
    };
    (@collect $tr:expr, [ $($arms:expr),* ] $cmd:expr $(,)?) => {
        $crate::first($tr, &[ $($arms,)* $crate::first!(@arm $cmd) ])
    };
    ($tr:expr, $($rest:tt)+) => {
        $crate::first!(@collect $tr, [] $($rest)+)
    };
}

#[macro_export]
macro_rules! chain {
    (@arm $($path:ident)::+ ( $($args:expr),* $(,)? )) => {
        &|tr| $($path)::+(tr $(, $args)*)
    };
    (@arm $cmd:expr) => {
        &$cmd
    };
    (@collect $tr:expr, [ $($arms:expr),* ] $($path:ident)::+ ( $($args:expr),* $(,)? ) , $($rest:tt)+) => {
        $crate::chain!(@collect $tr, [ $($arms,)* $crate::chain!(@arm $($path)::+($($args),*)) ] $($rest)+)
    };
    (@collect $tr:expr, [ $($arms:expr),* ] $($path:ident)::+ ( $($args:expr),* $(,)? ) $(,)?) => {
        $crate::chain($tr, &[ $($arms,)* $crate::chain!(@arm $($path)::+($($args),*)) ])
    };
    (@collect $tr:expr, [ $($arms:expr),* ] $cmd:expr , $($rest:tt)+) => {
        $crate::chain!(@collect $tr, [ $($arms,)* $crate::chain!(@arm $cmd) ] $($rest)+)
    };
    (@collect $tr:expr, [ $($arms:expr),* ] $cmd:expr $(,)?) => {
        $crate::chain($tr, &[ $($arms,)* $crate::chain!(@arm $cmd) ])
    };
    ($tr:expr, $($rest:tt)+) => {
        $crate::chain!(@collect $tr, [] $($rest)+)
    };
}

#[cfg(test)]
mod tests {
    use crate::CommandResult;
    use editor_transaction::Transaction;

    fn noop_command(tr: &mut Transaction) -> CommandResult {
        let _ = tr;
        Ok(true)
    }

    fn failing_command(tr: &mut Transaction) -> CommandResult {
        let _ = tr;
        Ok(false)
    }

    fn command_with_arg(tr: &mut Transaction, value: i32) -> CommandResult {
        let _ = (tr, value);
        Ok(true)
    }

    #[test]
    fn first_macro_bare_path() {
        let (state, ..) = editor_macros::state! {
            doc { root { paragraph { _t: text("hello") } } }
            selection: (_t, 0)
        };
        let mut tr = Transaction::new(&state);

        let result = first!(&mut tr, noop_command());
        assert_eq!(result.unwrap(), true);
    }

    #[test]
    fn first_macro_multiple_bare_paths() {
        let (state, ..) = editor_macros::state! {
            doc { root { paragraph { _t: text("hello") } } }
            selection: (_t, 0)
        };
        let mut tr = Transaction::new(&state);

        let result = first!(&mut tr, failing_command(), noop_command(),);
        assert_eq!(result.unwrap(), true);
    }

    #[test]
    fn first_macro_path_with_args() {
        let (state, ..) = editor_macros::state! {
            doc { root { paragraph { _t: text("hello") } } }
            selection: (_t, 0)
        };
        let mut tr = Transaction::new(&state);

        let result = first!(&mut tr, failing_command(), command_with_arg(42),);
        assert_eq!(result.unwrap(), true);
    }

    #[test]
    fn first_macro_closure() {
        let (state, ..) = editor_macros::state! {
            doc { root { paragraph { _t: text("hello") } } }
            selection: (_t, 0)
        };
        let mut tr = Transaction::new(&state);

        let result = first!(&mut tr, failing_command(), |_tr| Ok(true),);
        assert_eq!(result.unwrap(), true);
    }

    #[test]
    fn chain_macro_all_succeed() {
        let (state, ..) = editor_macros::state! {
            doc { root { paragraph { _t: text("hello") } } }
            selection: (_t, 0)
        };
        let mut tr = Transaction::new(&state);

        let result = chain!(&mut tr, noop_command(), command_with_arg(1), |_tr| Ok(true),);
        assert_eq!(result.unwrap(), true);
    }

    #[test]
    fn chain_macro_stops_on_failure() {
        let (state, ..) = editor_macros::state! {
            doc { root { paragraph { _t: text("hello") } } }
            selection: (_t, 0)
        };
        let mut tr = Transaction::new(&state);

        let result = chain!(&mut tr, noop_command(), failing_command(), noop_command(),);
        assert_eq!(result.unwrap(), false);
    }
}
