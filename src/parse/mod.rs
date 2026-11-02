use crate::{
    error::Error,
    lex::token::{TokenKind, keyword::Keyword, punc::Punctuation},
    src_code::span::Span,
};
use std::iter::{Filter, Peekable};

pub mod ast;

#[derive(Debug, Clone)]
pub struct Parser<I: Iterator<Item = (Span, TokenKind)>> {
    tokens: Peekable<Filter<I, fn(&I::Item) -> bool>>,
}

impl<I: Iterator<Item = (Span, TokenKind)>> Parser<I> {
    pub fn new<J>(tokens: J) -> Self
    where
        J: IntoIterator<IntoIter = I>,
    {
        let filter: fn(&I::Item) -> bool = |&(_, kind)| {
            !matches!(
                kind,
                TokenKind::Newline | TokenKind::WSpace | TokenKind::Comment
            )
        };
        Self {
            tokens: tokens.into_iter().filter(filter).peekable(),
        }
    }

    fn peek(&mut self) -> Option<&I::Item> {
        self.tokens.peek()
    }

    fn next(&mut self) -> Option<I::Item> {
        self.tokens.next()
    }

    fn next_if<P>(&mut self, p: P) -> Option<I::Item>
    where
        P: FnOnce(&I::Item) -> bool,
    {
        self.tokens.next_if(p)
    }

    fn sync(&mut self) {
        use Keyword::{
            Alt, Cat, Cord, Def, Done, Emit, Fail, For, Fun, Give, If, Let, Loop, Mem, Or, Pick,
            Prnt, Pvt, Rec, Rep, Skip, Sub, Sup, Uni,
        };
        while let Some((_, kind)) = self.next() {
            // end of current statement
            if kind == TokenKind::Punc(Punctuation::Semi) {
                break;
            }

            // start of new statement/definition
            if self.peek().is_some_and(|(_, kind)| {
                matches!(
                    kind,
                    TokenKind::Keyword(
                        Rec | Sup
                            | Cat
                            | Alt
                            | Sub
                            | Def
                            | Fun
                            | Mem
                            | Let
                            | Uni
                            | Pvt
                            | If
                            | Or
                            | Pick
                            | Rep
                            | For
                            | Loop
                            | Cord
                            | Done
                            | Skip
                            | Give
                            | Fail
                            | Emit
                            | Prnt
                    )
                )
            }) {
                break;
            }
        }
    }
}

pub fn parse<I>(tokens: I) -> Result<ast::Program, Vec<Error>>
where
    I: IntoIterator<Item = (Span, TokenKind)>,
{
    Parser::new(tokens).program().map_err(Vec::from_iter)
}

impl<I: Iterator<Item = (Span, TokenKind)>> Parser<I> {
    /// `program -> ( statement )*`
    pub fn program(&mut self) -> Result<ast::Program, impl IntoIterator<Item = Error>> {
        let mut iter = std::iter::from_fn(|| {
            self.tokens
                .peek()
                .is_some()
                .then(|| self.declaration().inspect_err(|_| self.sync()))
        });
        let (lo, hi) = iter.size_hint();
        let mut decls = Vec::with_capacity(hi.unwrap_or(lo));
        while let Some(res) = iter.next() {
            match res {
                Ok(stmt) => decls.push(stmt),
                Err(first) => {
                    return Err(std::iter::once(first).chain(iter.filter_map(Result::err)));
                }
            }
        }
        Ok(ast::Program { decls })
    }

    /// `declaration -> var_declaration | defer_declaration | statement`
    pub fn declaration(&mut self) -> Result<ast::Declaration, Error> {
        todo!()
    }

    /// `assignment -> "=" expression`
    pub fn assignment(&mut self) -> Option<Result<ast::Assignment, Error>> {
        self.next_if(|(_, kind)| matches!(kind, TokenKind::Punc(Punctuation::Assign)))
            .map(|(op, _)| {
                Ok(ast::Assignment {
                    op,
                    expr: self.expression()?,
                })
            })
    }

    /// `var_declaration -> ( "uni" | "pvt" | "let" ) pattern ( type_annotation )? ( assignment )? ";"`
    pub fn var_declaration(&mut self) -> Option<Result<ast::DeclVar, Error>> {
        self.next_if(|(_, kind)| {
            matches!(
                kind,
                TokenKind::Keyword(Keyword::Uni | Keyword::Pvt | Keyword::Let)
            )
        })
        .map(|_| todo!())
    }

    /// `type_annotation -> ":" type_expression`
    pub fn type_annotation(&mut self) -> Option<Result<ast::TypeAnnotation, Error>> {
        self.next_if(|(_, kind)| matches!(kind, TokenKind::Punc(Punctuation::Colon)))
            .map(|(colon, _)| {
                Ok(ast::TypeAnnotation {
                    colon,
                    ty: self.type_expression()?,
                })
            })
    }

    /// `statement -> block_statement
    ///     | print_statement
    ///     | done_statement
    ///     | skip_statement
    ///     | if_statement
    ///     | loop_statement
    ///     | expression_statement`
    pub fn statement(&mut self) -> Result<ast::Statement, Error> {
        todo!()
    }

    /// `expression_statement -> assign ";"`
    pub fn expression_statement(&mut self) -> Result<ast::ExprStmt, Error> {
        todo!()
    }

    /// `print_statement -> "prnt" expression ";"`
    pub fn print_statement(&mut self) -> Option<Result<ast::PrintStmt, Error>> {
        todo!()
    }

    /// `defer_declaration -> "defer" ( defer_param )? block_statement`
    pub fn defer_declaration(&mut self) -> Option<Result<ast::DeclDefer, Error>> {
        todo!()
    }

    /// `defer_param -> "(" bind_pattern ")"`
    pub fn defer_param(&mut self) -> Option<Result<ast::DeferParam, Error>> {
        todo!()
    }

    /// `if_statement -> "if" expression block_statement ( or_statement )?`
    pub fn if_statement(&mut self) -> Option<Result<ast::IfStmt, Error>> {
        todo!()
    }

    /// `or_statement -> "or" or_body`
    pub fn or_statement(&mut self) -> Option<Result<ast::OrStmt, Error>> {
        todo!()
    }

    /// `or_body -> if_statement | block_statement`
    pub fn or_body(&mut self) -> Result<ast::OrBody, Error> {
        todo!()
    }

    /// `loop_block -> ( scope_label )? loop_block1`
    pub fn loop_block(&mut self) -> Option<Result<ast::LoopBlock, Error>> {
        todo!()
    }

    /// `label -> "." IDENTIFIER`
    pub fn label(&mut self) -> Option<Result<ast::Label, Error>> {
        todo!()
    }

    /// `scope_label -> label ":"`
    pub fn scope_label(&mut self) -> Option<Result<ast::ScopeLabel, Error>> {
        todo!()
    }

    /// `loop_block1 -> ( "loop" | "cord" | "for" bind_pattern "in" expression | "rep" expression ) block_statement`
    pub fn loop_block1(
        &mut self,
        label: Option<ast::ScopeLabel>,
    ) -> Option<Result<ast::LoopBlock, Error>> {
        todo!()
    }

    /// `done_statement -> "done" ( label )? expression ";"`
    pub fn done_statement(&mut self) -> Option<Result<ast::DoneStmt, Error>> {
        todo!()
    }

    /// `skip_statement -> "skip" ( label )? ";"`
    pub fn skip_statement(&mut self) -> Option<Result<ast::SkipStmt, Error>> {
        todo!()
    }

    /// `block_statement -> "{" ( declaration )* "}"`
    pub fn block_statement(&mut self) -> Option<Result<ast::BlockStmt, Error>> {
        todo!()
    }

    /// `assign -> expression (
    ///       "="
    ///     | "+="
    ///     | "-="
    ///     | "*="
    ///     | "/="
    ///     | "%="
    ///     | "**="
    ///     | "<<="
    ///     | ">>="
    ///     | "[<<]="
    ///     | "[>>]="
    ///     | "|="
    ///     | "&="
    ///     | "^="
    ///     | "!|="
    ///     | "!&="
    ///     | "!^="
    /// ) expression | expression`
    fn assign(&mut self) -> Result<ast::Expr, Error> {
        todo!()
    }

    /// `expression -> if_block | loop_block | block_statement | or`
    pub fn expression(&mut self) -> Result<ast::Expr, Error> {
        todo!()
    }

    /// `or -> xor ( ("|" | "!|") xor )*`
    fn or(&mut self) -> Result<ast::Expr, Error> {
        todo!()
    }

    /// `xor -> and ( ("^" | "!^") and )*`
    fn xor(&mut self) -> Result<ast::Expr, Error> {
        todo!()
    }

    /// `and -> equality ( ("&" | "!&") equality )*`
    fn and(&mut self) -> Result<ast::Expr, Error> {
        todo!()
    }

    /// `equality -> comparison ( ("!=" | "==") comparison )*`
    fn equality(&mut self) -> Result<ast::Expr, Error> {
        todo!()
    }

    /// `comparison -> shift ( ( ">" | ">=" | "<" | "<=" ) shift )*`
    fn comparison(&mut self) -> Result<ast::Expr, Error> {
        todo!()
    }

    /// `shift -> term ( ( "<<" | ">>" | "[<<]" | "[>>]" ) term )*`
    fn shift(&mut self) -> Result<ast::Expr, Error> {
        todo!()
    }

    /// `term -> factor ( ( "+" | "-" ) factor )*`
    fn term(&mut self) -> Result<ast::Expr, Error> {
        todo!()
    }

    /// `factor -> unary_prefix ( ( "*" | "/" | "%" ) unary_prefix )*`
    fn factor(&mut self) -> Result<ast::Expr, Error> {
        todo!()
    }

    // TODO: `szof uint 3 ** 2` shouldn't be valid syntax
    /// `unary_prefix -> ( ( ( "!" | "!!" | "-" ) | "szof" ( type_expression )? ) )* exponent`
    fn unary_prefix(&mut self) -> Result<ast::Expr, Error> {
        todo!()
    }

    /// `exponent -> conversion ( "**" conversion )*`
    fn exponent(&mut self) -> Result<ast::Expr, Error> {
        todo!()
    }

    /// `conversion -> unary_postfix ( "-:>" | "=:>" ) type_expression`
    fn conversion(&mut self) -> Result<ast::Expr, Error> {
        todo!()
    }

    /// `unary_postfix -> primary ( "?" | "++" | "--" )*`
    fn unary_postfix(&mut self) -> Result<ast::Expr, Error> {
        todo!()
    }

    /// `primary -> literal | group | variable | type_expression`
    ///
    /// `type_expression`/`type_bound` are included as a variant of `primary` in order
    /// to allow type errors instead of just "wrong token" errors.
    fn primary(&mut self) -> Result<ast::Expr, Error> {
        todo!()
    }

    /// `directive -> "inpt"`
    fn directive(&mut self) -> Option<ast::Directive> {
        todo!()
    }

    /// `pattern -> IDENTIFIER`
    fn bind_pattern(&mut self) -> Option<ast::BindingPattern> {
        todo!()
    }

    /// `variable -> IDENTIFIER`
    fn variable(&mut self) -> Option<ast::Variable> {
        todo!()
    }

    /// `literal -> "true" | "fals" | "none" | BYTE | UINT | SINT | FLPT | CHAR | TEXT | MACRO`
    fn literal(&mut self) -> Option<ast::Literal> {
        todo!()
    }

    /// `sequence -> "[" expression "]"`
    fn sequence(&mut self) -> Option<Result<ast::Grouping, Error>> {
        todo!()
    }

    /// `group -> "(" expression ")"`
    fn group(&mut self) -> Option<Result<ast::Grouping, Error>> {
        todo!()
    }

    /// `generic_group -> "<" ( type_expression ( "," type_expression )* ( "," )? )? ">"`
    fn generic_group(&mut self) -> Option<Result<ast::GenericGroup, Error>> {
        todo!()
    }

    /// `type_expression -> ( ( "nevr" | "bool" | "uint" | "sint" | "flpt" | "char" | "text" | "fail" | IDENTIFIER ) "::" )*
    ///     ( "nevr" | "bool" | "uint" | "sint" | "flpt" | "char" | "text" | "fail" | IDENTIFIER )
    ///     ( generic_group )?
    ///     ( "|" ( "fail" | "none" | "nevr" ) )?`
    fn type_expression(&mut self) -> Result<ast::TypeExpr, Error> {
        todo!()
    }

    /// `iff_clause -> "<=>" type_bound`
    fn iff_clause(&mut self) -> Option<Result<ast::IffClause, Error>> {
        todo!()
    }

    /// `type_bound -> exact_bound | member_bound | shape_bound | expr_bound | grouped_bound | or_bound`
    fn type_bound(&mut self) -> Result<ast::TypeBound, Error> {
        todo!()
    }

    /// `exact_bound -> "only" type_expression`
    fn exact_bound(&mut self) -> Option<Result<ast::ExactTypeBound, Error>> {
        todo!()
    }

    /// `member_bound -> "has" IDENTIFIER ( colon_bound )?`
    fn member_bound(&mut self) -> Option<Result<ast::MemberBound, Error>> {
        todo!()
    }

    /// `type_bound -> ":" type_bound`
    fn colon_bound(&mut self) -> Option<Result<ast::ColonBound, Error>> {
        todo!()
    }

    /// `shape_bound -> "fun" "(" ( type_expression ( "," type_expression )* ( "," )? )? ")" "->" type_expression`
    fn shape_bound(&mut self) -> Option<Result<ast::ShapeBound, Error>> {
        todo!()
    }

    /// `expr_bound -> "for" IDENTIFIER ":" expression`
    fn expr_bound(&mut self) -> Option<Result<ast::ExprBound, Error>> {
        todo!()
    }

    /// `grouped_bound -> "(" type_bound ")"`
    fn grouped_bound(&mut self) -> Option<Result<ast::GroupedBound, Error>> {
        todo!()
    }

    /// `or_bound -> xor_bound ( "|" xor_bound )*`
    fn or_bound(&mut self) -> Result<ast::TypeBound, Error> {
        todo!()
    }

    /// `xor_bound -> and_bound ( "/" and_bound )*`
    fn xor_bound(&mut self) -> Result<ast::TypeBound, Error> {
        todo!()
    }

    /// `and_bound -> not_bound ( "+" not_bound )*`
    fn and_bound(&mut self) -> Result<ast::TypeBound, Error> {
        todo!()
    }

    /// `not_bound -> ( "!" )* type_bound`
    fn not_bound(&mut self) -> Result<ast::TypeBound, Error> {
        todo!()
    }
}
