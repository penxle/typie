macro_rules! assert_state_eq {
    ($actual:expr, $expected:expr) => {
        editor_state::assert_state_eq_impl(&$actual, &$expected)
    };
}

macro_rules! transact {
    ($initial:expr, |$tr:ident| $body:expr) => {{
        let initial: editor_state::State = $initial;
        #[allow(unused_mut)]
        let mut $tr = editor_transaction::Transaction::new(&initial);
        assert!($body.unwrap(), "command returned Ok(false)");
        $tr.commit()
    }};
}

macro_rules! transact_err {
    ($initial:expr, |$tr:ident| $body:expr) => {{
        let initial: editor_state::State = $initial;
        #[allow(unused_mut)]
        let mut $tr = editor_transaction::Transaction::new(&initial);
        $body.unwrap_err()
    }};
}

macro_rules! transact_fail {
    ($initial:expr, |$tr:ident| $body:expr) => {{
        let initial: editor_state::State = $initial;
        #[allow(unused_mut)]
        let mut $tr = editor_transaction::Transaction::new(&initial);
        assert!(
            !$body.unwrap(),
            "command returned Ok(true), expected Ok(false)"
        );
        $tr.commit()
    }};
}

pub(crate) use assert_state_eq;
pub(crate) use transact;
pub(crate) use transact_err;
pub(crate) use transact_fail;
