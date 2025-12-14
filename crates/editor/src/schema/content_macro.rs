#[macro_export]
macro_rules! content_expr {
    () => {
        $crate::schema::content::ContentExpr::Empty
    };

    ([$($item:tt),+ $(,)?]) => {
        $crate::schema::content::ContentExpr::Seq(vec![
            $(content_expr!(@el $item)),+
        ])
    };

    ($n:ident +) => {
        $crate::schema::content::ContentExpr::OneOrMore(Box::new(
            $crate::schema::content::ContentExpr::Single($crate::model::NodeType::$n)
        ))
    };
    ($n:ident *) => {
        $crate::schema::content::ContentExpr::ZeroOrMore(Box::new(
            $crate::schema::content::ContentExpr::Single($crate::model::NodeType::$n)
        ))
    };
    ($n:ident ?) => {
        $crate::schema::content::ContentExpr::Optional(Box::new(
            $crate::schema::content::ContentExpr::Single($crate::model::NodeType::$n)
        ))
    };

    (($($n:ident)|+) +) => {
        $crate::schema::content::ContentExpr::OneOrMore(Box::new(
            $crate::schema::content::ContentExpr::Choice(vec![
                $($crate::schema::content::ContentExpr::Single($crate::model::NodeType::$n)),+
            ])
        ))
    };
    (($($n:ident)|+) *) => {
        $crate::schema::content::ContentExpr::ZeroOrMore(Box::new(
            $crate::schema::content::ContentExpr::Choice(vec![
                $($crate::schema::content::ContentExpr::Single($crate::model::NodeType::$n)),+
            ])
        ))
    };
    (($($n:ident)|+) ?) => {
        $crate::schema::content::ContentExpr::Optional(Box::new(
            $crate::schema::content::ContentExpr::Choice(vec![
                $($crate::schema::content::ContentExpr::Single($crate::model::NodeType::$n)),+
            ])
        ))
    };

    ($n:ident) => {
        $crate::schema::content::ContentExpr::Single($crate::model::NodeType::$n)
    };
    (($($n:ident)|+)) => {
        $crate::schema::content::ContentExpr::Choice(vec![
            $($crate::schema::content::ContentExpr::Single($crate::model::NodeType::$n)),+
        ])
    };

    (@el ($n:ident +)) => {
        $crate::schema::content::ContentExpr::OneOrMore(Box::new(
            $crate::schema::content::ContentExpr::Single($crate::model::NodeType::$n)
        ))
    };
    (@el ($n:ident *)) => {
        $crate::schema::content::ContentExpr::ZeroOrMore(Box::new(
            $crate::schema::content::ContentExpr::Single($crate::model::NodeType::$n)
        ))
    };
    (@el ($n:ident ?)) => {
        $crate::schema::content::ContentExpr::Optional(Box::new(
            $crate::schema::content::ContentExpr::Single($crate::model::NodeType::$n)
        ))
    };
    (@el (($($n:ident)|+) +)) => {
        $crate::schema::content::ContentExpr::OneOrMore(Box::new(
            $crate::schema::content::ContentExpr::Choice(vec![
                $($crate::schema::content::ContentExpr::Single($crate::model::NodeType::$n)),+
            ])
        ))
    };
    (@el (($($n:ident)|+) *)) => {
        $crate::schema::content::ContentExpr::ZeroOrMore(Box::new(
            $crate::schema::content::ContentExpr::Choice(vec![
                $($crate::schema::content::ContentExpr::Single($crate::model::NodeType::$n)),+
            ])
        ))
    };
    (@el (($($n:ident)|+) ?)) => {
        $crate::schema::content::ContentExpr::Optional(Box::new(
            $crate::schema::content::ContentExpr::Choice(vec![
                $($crate::schema::content::ContentExpr::Single($crate::model::NodeType::$n)),+
            ])
        ))
    };
    (@el ($n:ident)) => {
        $crate::schema::content::ContentExpr::Single($crate::model::NodeType::$n)
    };
    (@el (($($n:ident)|+))) => {
        $crate::schema::content::ContentExpr::Choice(vec![
            $($crate::schema::content::ContentExpr::Single($crate::model::NodeType::$n)),+
        ])
    };
}
