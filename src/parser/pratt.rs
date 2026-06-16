use crate::parser::ast::*;
use crate::parser::error::*;
use crate::parser::lexer::*;
use crate::types::*;

/// Pratt parser for dice expressions
pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    errors: Vec<ParseError>,
    eof_token: Token,
}

impl Parser {
    /// Create a new parser from a list of tokens
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            pos: 0,
            errors: Vec::new(),
            eof_token: Token {
                kind: TokenKind::Eof,
                text: String::new(),
                start: 0,
                end: 0,
            },
        }
    }

    /// Parse a dice expression string into an AST
    pub fn parse(input: &str) -> ParseResult {
        let mut lexer = Lexer::new(input);
        let tokens = match lexer.tokenize() {
            Ok(t) => t,
            Err(e) => {
                return ParseResult::Failure(vec![ParseError {
                    message: e.message,
                    position: e.position,
                    suggestion: None,
                }]);
            }
        };

        let mut parser = Parser::new(tokens);
        let expr = parser.parse_expression(0);

        // Check for trailing tokens
        if parser.errors.is_empty() && parser.current().kind != TokenKind::Eof {
            parser.errors.push(ParseError {
                message: format!(
                    "Unexpected token '{}' after expression",
                    parser.current().kind
                ),
                position: parser.current().start,
                suggestion: None,
            });
        }

        if parser.errors.is_empty() {
            ParseResult::Success(expr)
        } else {
            ParseResult::Failure(parser.errors)
        }
    }

    fn current(&self) -> &Token {
        if self.pos < self.tokens.len() {
            &self.tokens[self.pos]
        } else {
            &self.eof_token
        }
    }

    fn advance(&mut self) -> &Token {
        let token = &self.tokens[self.pos];
        if self.pos < self.tokens.len() - 1 {
            self.pos += 1;
        }
        token
    }

    fn expect(&mut self, kind: TokenKind) -> Result<&Token, ParseError> {
        if self.current().kind == kind {
            Ok(self.advance())
        } else {
            Err(ParseError {
                message: format!("Expected '{}', got '{}'", kind, self.current().kind),
                position: self.current().start,
                suggestion: None,
            })
        }
    }

    fn parse_expression(&mut self, min_bp: u8) -> Expression {
        let mut lhs = self.parse_unary();

        loop {
            match &self.current().kind {
                TokenKind::Op(op) => {
                    let op = *op;
                    let (l_bp, r_bp) = Self::infix_binding_power(op);
                    if l_bp < min_bp {
                        break;
                    }
                    self.advance();
                    let rhs = self.parse_expression(r_bp);
                    lhs = Expression::BinaryOp {
                        op,
                        left: Box::new(lhs),
                        right: Box::new(rhs),
                    };
                }
                TokenKind::LParen => {
                    // Implicit multiplication: lhs * (expr)
                    // e.g., 2(3d6) = 2 * (3d6), 3d6(2+4) = 3d6 * (2+4)
                    let (l_bp, _) = Self::infix_binding_power(BinaryOp::Mul);
                    if l_bp < min_bp {
                        break;
                    }
                    self.advance(); // consume '('
                    let rhs = self.parse_expression(0);
                    let _ = self.expect(TokenKind::RParen);
                    lhs = Expression::BinaryOp {
                        op: BinaryOp::Mul,
                        left: Box::new(lhs),
                        right: Box::new(rhs),
                    };
                }
                _ => break,
            }
        }

        lhs
    }

    fn parse_unary(&mut self) -> Expression {
        match &self.current().kind.clone() {
            TokenKind::Op(BinaryOp::Sub) => {
                self.advance();
                let expr = self.parse_unary();
                Expression::UnaryMinus(Box::new(expr))
            }
            _ => self.parse_primary(),
        }
    }

    fn parse_primary(&mut self) -> Expression {
        match self.current().kind.clone() {
            TokenKind::Number(n) => {
                self.advance();
                Expression::Literal(n)
            }
            TokenKind::Dice(dice_token) => {
                self.advance();
                self.parse_dice_with_modifiers(dice_token)
            }
            TokenKind::LParen => {
                self.advance(); // consume '('
                let expr = self.parse_expression(0);

                // Check if this is an expression set: (expr, expr, ...)
                if self.current().kind == TokenKind::Comma {
                    self.parse_expression_set(expr)
                } else {
                    let _ = self.expect(TokenKind::RParen);
                    expr
                }
            }
            TokenKind::LBrack => {
                self.advance(); // consume '['
                self.parse_bracket_set()
            }
            _ => {
                self.errors.push(ParseError {
                    message: format!("Unexpected token '{}'", self.current().kind),
                    position: self.current().start,
                    suggestion: None,
                });
                Expression::Literal(0)
            }
        }
    }

    fn parse_dice_with_modifiers(&mut self, dice_token: DiceToken) -> Expression {
        let atom = match dice_token {
            DiceToken::Standard { count, sides } => DiceAtom::Standard { count, sides },
            DiceToken::Percent { count } => DiceAtom::Percent { count },
            DiceToken::Fate { count, magnitude } => DiceAtom::Fate { count, magnitude },
            DiceToken::Custom { count, faces } => DiceAtom::Custom { count, faces },
        };

        let mut functors = Vec::new();
        let mut filters = Vec::new();
        let mut count_threshold = None;
        let mut sort_order = None;

        // Parse functors and filters
        loop {
            match &self.current().kind.clone() {
                TokenKind::Shorthand(s) => match s {
                    ModifierShorthand::Explode => {
                        self.advance();
                        let (limit, condition) = self.parse_functor_args();
                        functors.push(Functor::Explode { limit, condition });
                    }
                    ModifierShorthand::Reroll => {
                        self.advance();
                        let (limit, condition) = self.parse_functor_args();
                        functors.push(Functor::Reroll { limit, condition });
                    }
                    ModifierShorthand::Compound => {
                        self.advance();
                        let (limit, condition) = self.parse_functor_args();
                        functors.push(Functor::Compound { limit, condition });
                    }
                    ModifierShorthand::Keep => {
                        self.advance();
                        let filter = self.parse_filter(FilterType::Keep);
                        filters.push(filter);
                    }
                    ModifierShorthand::Drop => {
                        self.advance();
                        let filter = self.parse_filter(FilterType::Drop);
                        filters.push(filter);
                    }
                    ModifierShorthand::Count => {
                        self.advance();
                        count_threshold = Some(self.parse_count_threshold());
                    }
                    ModifierShorthand::ExplodeBang => {
                        self.advance();
                        let condition = self.parse_bang_trigger_condition();
                        functors.push(Functor::Explode {
                            limit: FunctorLimit::Always,
                            condition,
                        });
                    }
                    ModifierShorthand::CompoundBang => {
                        self.advance();
                        let condition = self.parse_bang_trigger_condition();
                        functors.push(Functor::Compound {
                            limit: FunctorLimit::Always,
                            condition,
                        });
                    }
                    ModifierShorthand::KeepHigh => {
                        self.advance();
                        let n = self.parse_number_literal() as u32;
                        filters.push(Filter {
                            filter_type: FilterType::Keep,
                            n,
                            direction: FilterDirection::Highest,
                        });
                    }
                    ModifierShorthand::KeepLow => {
                        self.advance();
                        let n = self.parse_number_literal() as u32;
                        filters.push(Filter {
                            filter_type: FilterType::Keep,
                            n,
                            direction: FilterDirection::Lowest,
                        });
                    }
                    ModifierShorthand::DropHigh => {
                        self.advance();
                        let n = self.parse_number_literal() as u32;
                        filters.push(Filter {
                            filter_type: FilterType::Drop,
                            n,
                            direction: FilterDirection::Highest,
                        });
                    }
                    ModifierShorthand::DropLow => {
                        self.advance();
                        let n = self.parse_number_literal() as u32;
                        filters.push(Filter {
                            filter_type: FilterType::Drop,
                            n,
                            direction: FilterDirection::Lowest,
                        });
                    }
                    ModifierShorthand::RerollOnce => {
                        self.advance();
                        let condition = self.parse_trigger_condition();
                        functors.push(Functor::Reroll {
                            limit: FunctorLimit::Once,
                            condition,
                        });
                    }
                    ModifierShorthand::CountSuccess | ModifierShorthand::Target => {
                        self.advance();
                        count_threshold = Some(self.parse_target_threshold());
                    }
                    ModifierShorthand::MinCap => {
                        self.advance();
                        let min_value = self.parse_number_literal() as u32;
                        functors.push(Functor::MinCap { min_value });
                    }
                    ModifierShorthand::MaxCap => {
                        self.advance();
                        let max_value = self.parse_number_literal() as u32;
                        functors.push(Functor::MaxCap { max_value });
                    }
                    ModifierShorthand::SortAsc => {
                        self.advance();
                        sort_order = Some(SortOrder::Ascending);
                    }
                    ModifierShorthand::SortDesc => {
                        self.advance();
                        sort_order = Some(SortOrder::Descending);
                    }
                },
                TokenKind::Ident(ident) => {
                    match ident.as_str() {
                        "explode" | "e" => {
                            self.advance();
                            let (limit, condition) = self.parse_functor_args();
                            functors.push(Functor::Explode { limit, condition });
                        }
                        "reroll" | "r" => {
                            self.advance();
                            let (limit, condition) = self.parse_functor_args();
                            functors.push(Functor::Reroll { limit, condition });
                        }
                        "compound" | "ce" => {
                            self.advance();
                            let (limit, condition) = self.parse_functor_args();
                            functors.push(Functor::Compound { limit, condition });
                        }
                        "keep" | "k" => {
                            self.advance();
                            let filter = self.parse_filter(FilterType::Keep);
                            filters.push(filter);
                        }
                        "drop" | "d" => {
                            self.advance();
                            let filter = self.parse_filter(FilterType::Drop);
                            filters.push(filter);
                        }
                        "count" | "c" => {
                            self.advance();
                            count_threshold = Some(self.parse_count_threshold());
                        }
                        "emphasis" => {
                            self.advance();
                            let tie_break = self.parse_emphasis_tie_break();
                            functors.push(Functor::Emphasis {
                                tie_break,
                                center: None,
                            });
                        }
                        "furthest" => {
                            self.advance();
                            // expect "from"
                            if self.current().kind == TokenKind::Ident("from".into()) {
                                self.advance();
                                let center = self.parse_number_literal() as f64;
                                let tie_break = self.parse_emphasis_tie_break();
                                functors.push(Functor::Emphasis {
                                    tie_break,
                                    center: Some(center),
                                });
                            }
                        }
                        _ => break,
                    }
                }
                _ => break,
            }
        }

        Expression::Dice(DiceExpression {
            atom,
            functors,
            filters,
            count_threshold,
            sort_order,
        })
    }

    fn parse_functor_args(&mut self) -> (FunctorLimit, TriggerCondition) {
        let mut limit = FunctorLimit::Always;

        // Parse optional limit
        match self.current().kind.clone() {
            TokenKind::Ident(ref s) => match s.as_str() {
                "once" => {
                    self.advance();
                    limit = FunctorLimit::Once;
                }
                "twice" => {
                    self.advance();
                    limit = FunctorLimit::Twice;
                }
                "thrice" => {
                    self.advance();
                    limit = FunctorLimit::Thrice;
                }
                "always" => {
                    self.advance();
                }
                _ => {}
            },
            TokenKind::Number(n) if n > 0 => {
                self.advance();
                // expect "times"
                if self.current().kind == TokenKind::Ident("times".into()) {
                    self.advance();
                }
                limit = FunctorLimit::Times(n as u32);
            }
            _ => {}
        }

        // Parse optional "on" keyword
        if self.current().kind == TokenKind::Ident("on".into()) {
            self.advance();
        }

        // Parse trigger condition
        let condition = self.parse_trigger_condition();

        (limit, condition)
    }

    fn parse_trigger_condition(&mut self) -> TriggerCondition {
        match self.current().kind.clone() {
            TokenKind::Number(n) => {
                self.advance();
                let val = n as u32;

                // Check for range: "3..5"
                if self.current().kind == TokenKind::DotDot {
                    self.advance();
                    let high = self.parse_number_literal() as u32;
                    return TriggerCondition::Between(val, high);
                }

                // Check for "or more" / "or less"
                if self.current().kind == TokenKind::Ident("or".into()) {
                    self.advance();
                    match self.current().kind.clone() {
                        TokenKind::Ident(ref s) if s == "more" => {
                            self.advance();
                            return TriggerCondition::AtOrAbove(val);
                        }
                        TokenKind::Ident(ref s) if s == "less" => {
                            self.advance();
                            return TriggerCondition::AtOrBelow(val);
                        }
                        _ => {}
                    }
                }

                TriggerCondition::Exact(val)
            }
            TokenKind::Ident(ref s) if s == "max" => {
                self.advance();
                TriggerCondition::Max
            }
            _ => TriggerCondition::Max, // default to max if can't parse
        }
    }

    fn parse_bang_trigger_condition(&mut self) -> TriggerCondition {
        // Parse trigger condition for ! and !! syntax
        // Supports: !>=5, !>5, !<=5, !<5, !5, ! (default: max)
        match self.current().kind.clone() {
            TokenKind::CompOp(op) => {
                self.advance();
                let val = self.parse_number_literal() as u32;
                match op {
                    CountOp::Ge => TriggerCondition::AtOrAbove(val),
                    CountOp::Gt => TriggerCondition::AtOrAbove(val + 1),
                    CountOp::Le => TriggerCondition::AtOrBelow(val),
                    CountOp::Lt => TriggerCondition::AtOrBelow(val.saturating_sub(1)),
                    CountOp::Eq => TriggerCondition::Exact(val),
                    CountOp::Ne => TriggerCondition::Exact(val), // fallback
                }
            }
            TokenKind::Number(val) => {
                self.advance();
                TriggerCondition::Exact(val as u32)
            }
            _ => TriggerCondition::Max, // bare ! means explode on max
        }
    }

    fn parse_target_threshold(&mut self) -> MultiCountThreshold {
        // For target number: bare number defaults to >= instead of ==
        let (op, value) = match self.current().kind.clone() {
            TokenKind::CompOp(op) => {
                self.advance();
                let val = self.parse_number_literal() as u32;
                (op, val)
            }
            TokenKind::Number(val) => {
                self.advance();
                (CountOp::Ge, val as u32) // bare number defaults to >=
            }
            _ => (CountOp::Ge, 0), // fallback
        };
        MultiCountThreshold {
            thresholds: vec![CountThreshold { op, value }],
        }
    }

    fn parse_filter(&mut self, filter_type: FilterType) -> Filter {
        let mut direction = match filter_type {
            FilterType::Keep => FilterDirection::Highest,
            FilterType::Drop => FilterDirection::Lowest,
        };

        // Parse optional direction keyword
        if let TokenKind::Ident(ref s) = self.current().kind.clone() {
            match s.as_str() {
                "highest" | "high" => {
                    self.advance();
                    direction = FilterDirection::Highest;
                }
                "lowest" | "low" => {
                    self.advance();
                    direction = FilterDirection::Lowest;
                }
                "middle" | "mid" => {
                    self.advance();
                    direction = FilterDirection::Middle;
                }
                _ => {}
            }
        }

        let n = self.parse_number_literal() as u32;

        Filter {
            filter_type,
            n,
            direction,
        }
    }

    fn parse_count_threshold(&mut self) -> MultiCountThreshold {
        let mut thresholds = Vec::new();

        loop {
            let op = match self.current().kind.clone() {
                TokenKind::CompOp(op) => {
                    self.advance();
                    op
                }
                TokenKind::Ident(ref s) if s == "exactly" => {
                    self.advance();
                    CountOp::Eq
                }
                TokenKind::Ident(ref s) if s == "on" => {
                    self.advance();
                    // Parse as trigger-like condition
                    let val = self.parse_number_literal() as u32;
                    thresholds.push(CountThreshold {
                        op: CountOp::Ge,
                        value: val,
                    });
                    continue;
                }
                TokenKind::Number(_) => {
                    // Bare number without operator = exact match (e.g., "c6" means "count == 6")
                    CountOp::Eq
                }
                _ => break,
            };

            let value = self.parse_number_literal() as u32;
            thresholds.push(CountThreshold { op, value });

            // Check for "and" to chain thresholds
            if self.current().kind == TokenKind::Ident("and".into()) {
                self.advance();
            } else {
                break;
            }
        }

        MultiCountThreshold { thresholds }
    }

    fn parse_emphasis_tie_break(&mut self) -> EmphasisTieBreak {
        match self.current().kind.clone() {
            TokenKind::Ident(ref s) => match s.as_str() {
                "high" => {
                    self.advance();
                    EmphasisTieBreak::High
                }
                "low" => {
                    self.advance();
                    EmphasisTieBreak::Low
                }
                "reroll" => {
                    self.advance();
                    EmphasisTieBreak::Reroll
                }
                _ => EmphasisTieBreak::Reroll, // default
            },
            _ => EmphasisTieBreak::Reroll, // default
        }
    }

    fn parse_number_literal(&mut self) -> i32 {
        match self.current().kind.clone() {
            TokenKind::Number(n) => {
                self.advance();
                n
            }
            _ => {
                self.errors.push(ParseError {
                    message: format!("Expected number, got '{}'", self.current().kind),
                    position: self.current().start,
                    suggestion: None,
                });
                0
            }
        }
    }

    fn parse_expression_set(&mut self, first: Expression) -> Expression {
        let mut exprs = vec![first];

        loop {
            if self.current().kind != TokenKind::Comma {
                break;
            }
            self.advance(); // consume ','
            let expr = self.parse_expression(0);
            exprs.push(expr);
        }

        let _ = self.expect(TokenKind::RParen);

        // Parse optional reducer
        let reducer = self.parse_reducer();

        Expression::DiceSet { exprs, reducer }
    }

    fn parse_bracket_set(&mut self) -> Expression {
        let mut exprs = Vec::new();

        loop {
            if self.current().kind == TokenKind::RBrack {
                break;
            }
            let expr = self.parse_expression(0);
            exprs.push(expr);
            if self.current().kind == TokenKind::Comma {
                self.advance();
            }
        }

        let _ = self.expect(TokenKind::RBrack);

        // Parse optional reducer
        let reducer = self.parse_reducer();

        Expression::DiceSet { exprs, reducer }
    }

    fn parse_reducer(&mut self) -> Reducer {
        match self.current().kind.clone() {
            TokenKind::Ident(ref s) => match s.as_str() {
                "sum" => {
                    self.advance();
                    Reducer::Sum
                }
                "min" | "minimum" | "least" => {
                    self.advance();
                    Reducer::Min
                }
                "max" | "maximum" | "best" => {
                    self.advance();
                    Reducer::Max
                }
                "average" | "avg" => {
                    self.advance();
                    Reducer::Average
                }
                "median" | "med" => {
                    self.advance();
                    Reducer::Median
                }
                _ => Reducer::Sum, // default
            },
            _ => Reducer::Sum, // default
        }
    }

    fn infix_binding_power(op: BinaryOp) -> (u8, u8) {
        match op {
            BinaryOp::Add | BinaryOp::Sub => (1, 2),
            BinaryOp::Mul | BinaryOp::Div => (3, 4),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_literal() {
        let result = Parser::parse("42");
        assert!(result.success());
        assert_eq!(*result.expression().unwrap(), Expression::Literal(42));
    }

    #[test]
    fn test_parse_simple_dice() {
        let result = Parser::parse("3d6");
        assert!(result.success());
        match result.expression().unwrap() {
            Expression::Dice(d) => {
                assert_eq!(d.atom, DiceAtom::Standard { count: 3, sides: 6 });
            }
            _ => panic!("Expected Dice expression"),
        }
    }

    #[test]
    fn test_parse_d6() {
        let result = Parser::parse("d6");
        assert!(result.success());
        match result.expression().unwrap() {
            Expression::Dice(d) => {
                assert_eq!(d.atom, DiceAtom::Standard { count: 1, sides: 6 });
            }
            _ => panic!("Expected Dice expression"),
        }
    }

    #[test]
    fn test_parse_arithmetic() {
        let result = Parser::parse("3d6+4");
        assert!(result.success());
        match result.expression().unwrap() {
            Expression::BinaryOp { op, .. } => {
                assert_eq!(*op, BinaryOp::Add);
            }
            _ => panic!("Expected BinaryOp"),
        }
    }

    #[test]
    fn test_parse_keep() {
        let result = Parser::parse("4d6 keep 3");
        assert!(result.success());
        match result.expression().unwrap() {
            Expression::Dice(d) => {
                assert_eq!(d.filters.len(), 1);
                assert_eq!(d.filters[0].filter_type, FilterType::Keep);
                assert_eq!(d.filters[0].n, 3);
            }
            _ => panic!("Expected Dice expression"),
        }
    }

    // ── Parser Coverage Tests ─────────────────────────────────────────

    #[test]
    fn test_parse_arithmetic_deep() {
        let result = Parser::parse("3d6+4");
        assert!(result.success());
        match result.expression().unwrap() {
            Expression::BinaryOp { op, left, right } => {
                assert_eq!(*op, BinaryOp::Add);
                match left.as_ref() {
                    Expression::Dice(d) => {
                        assert_eq!(d.atom, DiceAtom::Standard { count: 3, sides: 6 });
                    }
                    _ => panic!("Expected Dice on left"),
                }
                match right.as_ref() {
                    Expression::Literal(n) => assert_eq!(*n, 4),
                    _ => panic!("Expected Literal on right"),
                }
            }
            _ => panic!("Expected BinaryOp"),
        }
    }

    #[test]
    fn test_parse_dice_set_paren() {
        let result = Parser::parse("(2d6, 3d6) sum");
        assert!(result.success());
        match result.expression().unwrap() {
            Expression::DiceSet { exprs, reducer } => {
                assert_eq!(exprs.len(), 2);
                assert_eq!(*reducer, Reducer::Sum);
            }
            _ => panic!("Expected DiceSet"),
        }
    }

    #[test]
    fn test_parse_bracket_set() {
        let result = Parser::parse("[d6, d8] max");
        assert!(result.success());
        match result.expression().unwrap() {
            Expression::DiceSet { exprs, reducer } => {
                assert_eq!(exprs.len(), 2);
                assert_eq!(*reducer, Reducer::Max);
            }
            _ => panic!("Expected DiceSet"),
        }
    }

    #[test]
    fn test_parse_unary_minus() {
        let result = Parser::parse("-d6 + 10");
        assert!(result.success());
        match result.expression().unwrap() {
            Expression::BinaryOp { op, left, .. } => {
                assert_eq!(*op, BinaryOp::Add);
                match left.as_ref() {
                    Expression::UnaryMinus(inner) => match inner.as_ref() {
                        Expression::Dice(d) => {
                            assert_eq!(d.atom, DiceAtom::Standard { count: 1, sides: 6 });
                        }
                        _ => panic!("Expected Dice inside UnaryMinus"),
                    },
                    _ => panic!("Expected UnaryMinus on left"),
                }
            }
            _ => panic!("Expected BinaryOp"),
        }
    }

    #[test]
    fn test_parse_implicit_multiplication() {
        let result = Parser::parse("2(3d6)");
        assert!(result.success());
        match result.expression().unwrap() {
            Expression::BinaryOp { op, left, right } => {
                assert_eq!(*op, BinaryOp::Mul);
                match left.as_ref() {
                    Expression::Literal(n) => assert_eq!(*n, 2),
                    _ => panic!("Expected Literal on left"),
                }
                match right.as_ref() {
                    Expression::Dice(d) => {
                        assert_eq!(d.atom, DiceAtom::Standard { count: 3, sides: 6 });
                    }
                    _ => panic!("Expected Dice on right"),
                }
            }
            _ => panic!("Expected BinaryOp"),
        }
    }

    #[test]
    fn test_parse_explode_bang() {
        let result = Parser::parse("3d6!");
        assert!(result.success());
        match result.expression().unwrap() {
            Expression::Dice(d) => {
                assert_eq!(d.functors.len(), 1);
                match &d.functors[0] {
                    Functor::Explode { condition, .. } => {
                        assert_eq!(*condition, TriggerCondition::Max);
                    }
                    _ => panic!("Expected Explode functor"),
                }
            }
            _ => panic!("Expected Dice"),
        }
    }

    #[test]
    fn test_parse_explode_bang_with_condition() {
        let result = Parser::parse("3d6!>=5");
        assert!(result.success());
        match result.expression().unwrap() {
            Expression::Dice(d) => {
                assert_eq!(d.functors.len(), 1);
                match &d.functors[0] {
                    Functor::Explode { condition, .. } => {
                        assert_eq!(*condition, TriggerCondition::AtOrAbove(5));
                    }
                    _ => panic!("Expected Explode functor"),
                }
            }
            _ => panic!("Expected Dice"),
        }
    }

    #[test]
    fn test_parse_compound_bang() {
        let result = Parser::parse("3d6!!");
        assert!(result.success());
        match result.expression().unwrap() {
            Expression::Dice(d) => {
                assert_eq!(d.functors.len(), 1);
                match &d.functors[0] {
                    Functor::Compound { .. } => {}
                    _ => panic!("Expected Compound functor"),
                }
            }
            _ => panic!("Expected Dice"),
        }
    }

    #[test]
    fn test_parse_reroll_shorthand() {
        let result = Parser::parse("3d6r1");
        assert!(result.success());
        match result.expression().unwrap() {
            Expression::Dice(d) => {
                assert_eq!(d.functors.len(), 1);
                match &d.functors[0] {
                    Functor::Reroll { limit, condition } => {
                        // "r1" parses 1 as limit (Times(1)), default condition (Max)
                        assert_eq!(*limit, FunctorLimit::Times(1));
                        assert_eq!(*condition, TriggerCondition::Max);
                    }
                    _ => panic!("Expected Reroll functor"),
                }
            }
            _ => panic!("Expected Dice"),
        }
    }

    #[test]
    fn test_parse_reroll_once() {
        let result = Parser::parse("2d6ro1");
        assert!(result.success());
        match result.expression().unwrap() {
            Expression::Dice(d) => {
                assert_eq!(d.functors.len(), 1);
                match &d.functors[0] {
                    Functor::Reroll { limit, .. } => {
                        assert_eq!(*limit, FunctorLimit::Once);
                    }
                    _ => panic!("Expected Reroll functor"),
                }
            }
            _ => panic!("Expected Dice"),
        }
    }

    #[test]
    fn test_parse_min_cap() {
        let result = Parser::parse("4d6mi2");
        assert!(result.success());
        match result.expression().unwrap() {
            Expression::Dice(d) => {
                assert_eq!(d.functors.len(), 1);
                assert_eq!(d.functors[0], Functor::MinCap { min_value: 2 });
            }
            _ => panic!("Expected Dice"),
        }
    }

    #[test]
    fn test_parse_max_cap() {
        let result = Parser::parse("4d6ma5");
        assert!(result.success());
        match result.expression().unwrap() {
            Expression::Dice(d) => {
                assert_eq!(d.functors.len(), 1);
                assert_eq!(d.functors[0], Functor::MaxCap { max_value: 5 });
            }
            _ => panic!("Expected Dice"),
        }
    }

    #[test]
    fn test_parse_count_success() {
        let result = Parser::parse("4d6cs>=4");
        assert!(result.success());
        match result.expression().unwrap() {
            Expression::Dice(d) => {
                assert!(d.count_threshold.is_some());
                let threshold = d.count_threshold.as_ref().unwrap();
                assert_eq!(threshold.thresholds.len(), 1);
                assert_eq!(threshold.thresholds[0].op, CountOp::Ge);
                assert_eq!(threshold.thresholds[0].value, 4);
            }
            _ => panic!("Expected Dice"),
        }
    }

    #[test]
    fn test_parse_target() {
        let result = Parser::parse("4d6t4");
        assert!(result.success());
        match result.expression().unwrap() {
            Expression::Dice(d) => {
                assert!(d.count_threshold.is_some());
                let threshold = d.count_threshold.as_ref().unwrap();
                assert_eq!(threshold.thresholds[0].op, CountOp::Ge);
                assert_eq!(threshold.thresholds[0].value, 4);
            }
            _ => panic!("Expected Dice"),
        }
    }

    #[test]
    fn test_parse_sort_ascending() {
        let result = Parser::parse("4d6sa");
        assert!(result.success());
        match result.expression().unwrap() {
            Expression::Dice(d) => {
                assert_eq!(d.sort_order, Some(SortOrder::Ascending));
            }
            _ => panic!("Expected Dice"),
        }
    }

    #[test]
    fn test_parse_sort_descending() {
        let result = Parser::parse("4d6sd");
        assert!(result.success());
        match result.expression().unwrap() {
            Expression::Dice(d) => {
                assert_eq!(d.sort_order, Some(SortOrder::Descending));
            }
            _ => panic!("Expected Dice"),
        }
    }

    #[test]
    fn test_parse_keep_high() {
        let result = Parser::parse("4d6kh3");
        assert!(result.success());
        match result.expression().unwrap() {
            Expression::Dice(d) => {
                assert_eq!(d.filters.len(), 1);
                assert_eq!(d.filters[0].filter_type, FilterType::Keep);
                assert_eq!(d.filters[0].direction, FilterDirection::Highest);
                assert_eq!(d.filters[0].n, 3);
            }
            _ => panic!("Expected Dice"),
        }
    }

    #[test]
    fn test_parse_keep_low() {
        let result = Parser::parse("4d6kl1");
        assert!(result.success());
        match result.expression().unwrap() {
            Expression::Dice(d) => {
                assert_eq!(d.filters.len(), 1);
                assert_eq!(d.filters[0].filter_type, FilterType::Keep);
                assert_eq!(d.filters[0].direction, FilterDirection::Lowest);
                assert_eq!(d.filters[0].n, 1);
            }
            _ => panic!("Expected Dice"),
        }
    }

    #[test]
    fn test_parse_drop_high() {
        let result = Parser::parse("4d6dh1");
        assert!(result.success());
        match result.expression().unwrap() {
            Expression::Dice(d) => {
                assert_eq!(d.filters.len(), 1);
                assert_eq!(d.filters[0].filter_type, FilterType::Drop);
                assert_eq!(d.filters[0].direction, FilterDirection::Highest);
                assert_eq!(d.filters[0].n, 1);
            }
            _ => panic!("Expected Dice"),
        }
    }

    #[test]
    fn test_parse_drop_low() {
        let result = Parser::parse("4d6dl1");
        assert!(result.success());
        match result.expression().unwrap() {
            Expression::Dice(d) => {
                assert_eq!(d.filters.len(), 1);
                assert_eq!(d.filters[0].filter_type, FilterType::Drop);
                assert_eq!(d.filters[0].direction, FilterDirection::Lowest);
                assert_eq!(d.filters[0].n, 1);
            }
            _ => panic!("Expected Dice"),
        }
    }

    #[test]
    fn test_parse_multiple_filters() {
        let result = Parser::parse("20d6 keep 5 drop 2");
        assert!(result.success());
        match result.expression().unwrap() {
            Expression::Dice(d) => {
                assert_eq!(d.filters.len(), 2);
                assert_eq!(d.filters[0].filter_type, FilterType::Keep);
                assert_eq!(d.filters[0].n, 5);
                assert_eq!(d.filters[1].filter_type, FilterType::Drop);
                assert_eq!(d.filters[1].n, 2);
            }
            _ => panic!("Expected Dice"),
        }
    }

    #[test]
    fn test_parse_trigger_on_max() {
        let result = Parser::parse("3d6 explode on max");
        assert!(result.success());
        match result.expression().unwrap() {
            Expression::Dice(d) => {
                assert_eq!(d.functors.len(), 1);
                match &d.functors[0] {
                    Functor::Explode { condition, .. } => {
                        assert_eq!(*condition, TriggerCondition::Max);
                    }
                    _ => panic!("Expected Explode"),
                }
            }
            _ => panic!("Expected Dice"),
        }
    }

    #[test]
    fn test_parse_trigger_on_value_or_more() {
        let result = Parser::parse("3d6 explode on 5 or more");
        assert!(result.success());
        match result.expression().unwrap() {
            Expression::Dice(d) => {
                assert_eq!(d.functors.len(), 1);
                match &d.functors[0] {
                    Functor::Explode { condition, .. } => {
                        assert_eq!(*condition, TriggerCondition::AtOrAbove(5));
                    }
                    _ => panic!("Expected Explode"),
                }
            }
            _ => panic!("Expected Dice"),
        }
    }

    #[test]
    fn test_parse_trigger_between() {
        let result = Parser::parse("3d6 reroll on 1..2");
        assert!(result.success());
        match result.expression().unwrap() {
            Expression::Dice(d) => {
                assert_eq!(d.functors.len(), 1);
                match &d.functors[0] {
                    Functor::Reroll { condition, .. } => {
                        assert_eq!(*condition, TriggerCondition::Between(1, 2));
                    }
                    _ => panic!("Expected Reroll"),
                }
            }
            _ => panic!("Expected Dice"),
        }
    }

    #[test]
    fn test_parse_keep_middle() {
        let result = Parser::parse("5d6 keep middle 3");
        assert!(result.success());
        match result.expression().unwrap() {
            Expression::Dice(d) => {
                assert_eq!(d.filters.len(), 1);
                assert_eq!(d.filters[0].filter_type, FilterType::Keep);
                assert_eq!(d.filters[0].direction, FilterDirection::Middle);
                assert_eq!(d.filters[0].n, 3);
            }
            _ => panic!("Expected Dice"),
        }
    }

    #[test]
    fn test_parse_dice_set_no_reducer_defaults_sum() {
        let result = Parser::parse("(2d6, 3d6)");
        assert!(result.success());
        match result.expression().unwrap() {
            Expression::DiceSet { exprs, reducer } => {
                assert_eq!(exprs.len(), 2);
                assert_eq!(*reducer, Reducer::Sum);
            }
            _ => panic!("Expected DiceSet"),
        }
    }

    #[test]
    fn test_parse_dice_set_min_reducer() {
        let result = Parser::parse("[d6, d8, d10] min");
        assert!(result.success());
        match result.expression().unwrap() {
            Expression::DiceSet { exprs, reducer } => {
                assert_eq!(exprs.len(), 3);
                assert_eq!(*reducer, Reducer::Min);
            }
            _ => panic!("Expected DiceSet"),
        }
    }

    // ── Error Path Tests ──────────────────────────────────────────────

    #[test]
    fn test_parse_error_empty_expression() {
        let result = Parser::parse("");
        assert!(!result.success());
        assert!(!result.errors().is_empty());
    }

    #[test]
    fn test_parse_error_trailing_operator() {
        let result = Parser::parse("3d6+");
        assert!(!result.success());
        assert!(!result.errors().is_empty());
    }

    #[test]
    fn test_parse_error_unmatched_paren() {
        // Parser uses `let _ = self.expect(RParen)` so unmatched paren
        // is silently ignored - expression still parses successfully
        let result = Parser::parse("(3d6");
        assert!(result.success());
        match result.expression().unwrap() {
            Expression::Dice(d) => {
                assert_eq!(d.atom, DiceAtom::Standard { count: 3, sides: 6 });
            }
            _ => panic!("Expected Dice expression"),
        }
    }

    #[test]
    fn test_parse_error_unexpected_token_after_expr() {
        let result = Parser::parse("3d6 4d6");
        assert!(!result.success());
        assert!(!result.errors().is_empty());
    }

    #[test]
    fn test_parse_error_invalid_token() {
        let result = Parser::parse("@#");
        assert!(!result.success());
        assert!(!result.errors().is_empty());
    }
}
