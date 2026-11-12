use std::collections::HashMap;

use crate::token::Token;
use sir::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub message: String,
}

impl ParseError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

// todo: rewrite parse-error on thiserror
impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

pub type ParseResult<T> = Result<T, ParseError>;

pub struct Parser {
    tokens: Vec<Token>,
    cursor: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, cursor: 0 }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.cursor)
    }

    fn advance(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.cursor)?.clone();
        self.cursor += 1;
        Some(token)
    }

    fn expect(&mut self, expected: Token) -> ParseResult<()> {
        let token = self.advance();

        if token == Some(expected.clone()) {
            Ok(())
        } else {
            Err(ParseError::new(format!(
                "expected {expected:?}, got {token:?}"
            )))
        }
    }

    fn expect_ident(&mut self) -> ParseResult<String> {
        match self.advance() {
            Some(Token::Ident(name)) => Ok(name),
            token => Err(ParseError::new(format!(
                "expected identifier, got {token:?}"
            ))),
        }
    }

    fn expect_int(&mut self) -> ParseResult<i64> {
        match self.advance() {
            Some(Token::IntLiteral(value)) => Ok(value),
            token => Err(ParseError::new(format!("expected integer, got {token:?}"))),
        }
    }

    fn parse_type(&mut self) -> ParseResult<Type> {
        match self.advance() {
            Some(Token::I1) => Ok(Type::Bool),
            Some(Token::I8) => Ok(Type::Int8),
            Some(Token::I16) => Ok(Type::Int16),
            Some(Token::I32) => Ok(Type::Int32),
            Some(Token::I64) => Ok(Type::Int64),
            Some(Token::F32) => Ok(Type::Float32),
            Some(Token::F64) => Ok(Type::Float64),

            Some(Token::Ident(name)) if name == "void" => Ok(Type::Void),

            token => Err(ParseError::new(format!("expected type, got {token:?}"))),
        }
    }

    fn parse_arg_ref(&mut self, ctx: &FunctionParseContext) -> ParseResult<ArgRef> {
        match self.advance() {
            Some(Token::ArgRef(inner)) => match *inner {
                Token::Ident(name) => ctx.resolve_arg(&name),

                Token::IntLiteral(value) if value >= 0 => Ok(value as ArgRef),

                _ => Err(ParseError::new("invalid argument reference")),
            },

            token => Err(ParseError::new(format!(
                "expected argument reference, got {token:?}"
            ))),
        }
    }

    fn parse_local_ref(&mut self, ctx: &FunctionParseContext) -> ParseResult<LocalRef> {
        match self.advance() {
            Some(Token::LocalRef(inner)) => match *inner {
                Token::Ident(name) => ctx.resolve_local(&name),

                Token::IntLiteral(value) if value >= 0 => Ok(value as LocalRef),

                _ => Err(ParseError::new("invalid local reference")),
            },

            token => Err(ParseError::new(format!(
                "expected local reference, got {token:?}"
            ))),
        }
    }

    fn parse_constant(&mut self) -> ParseResult<Constant> {
        match self.advance() {
            Some(Token::IntLiteral(value)) => Ok(Constant::Int(value)),

            Some(Token::FloatLiteral(value)) => {
                let value = value
                    .parse::<f64>()
                    .map_err(|_| ParseError::new("invalid float constant"))?;

                Ok(Constant::Float(value))
            }

            token => Err(ParseError::new(format!("expected constant, got {token:?}"))),
        }
    }

    fn parse_predicate(&mut self) -> ParseResult<IntPredicate> {
        match self.advance() {
            Some(Token::Ieq) => Ok(IntPredicate::Eq),
            Some(Token::Ine) => Ok(IntPredicate::Ne),
            Some(Token::Islt) => Ok(IntPredicate::Slt),
            Some(Token::Isle) => Ok(IntPredicate::Sle),
            Some(Token::Isgt) => Ok(IntPredicate::Sgt),
            Some(Token::Isge) => Ok(IntPredicate::Sge),

            Some(Token::Iult) => Ok(IntPredicate::Ult),
            Some(Token::Iule) => Ok(IntPredicate::Ule),
            Some(Token::Iugt) => Ok(IntPredicate::Ugt),
            Some(Token::Iuge) => Ok(IntPredicate::Uge),

            token => Err(ParseError::new(format!(
                "expected integer predicate, got {token:?}"
            ))),
        }
    }

    fn parse_float_predicate(&mut self) -> ParseResult<FloatPredicate> {
        match self.advance() {
            Some(Token::Feq) => Ok(FloatPredicate::Eq),
            Some(Token::Fne) => Ok(FloatPredicate::Ne),
            Some(Token::Folt) => Ok(FloatPredicate::Olt),
            Some(Token::Fole) => Ok(FloatPredicate::Ole),
            Some(Token::Fogt) => Ok(FloatPredicate::Ogt),
            Some(Token::Foge) => Ok(FloatPredicate::Oge),

            Some(Token::Fult) => Ok(FloatPredicate::Ult),
            Some(Token::Fule) => Ok(FloatPredicate::Ule),
            Some(Token::Fugt) => Ok(FloatPredicate::Ugt),
            Some(Token::Fuge) => Ok(FloatPredicate::Uge),

            token => Err(ParseError::new(format!(
                "expected float predicate, got {token:?}"
            ))),
        }
    }
}

impl Parser {
    fn parse_function_signature(
        &mut self,
    ) -> ParseResult<(String, Vec<Type>, HashMap<String, ArgRef>, Type)> {
        self.expect(Token::Function)?;

        let name = self.expect_ident()?;

        self.expect(Token::LParen)?;

        let mut params = Vec::new();
        let mut args = HashMap::new();

        if self.peek() != Some(&Token::RParen) {
            loop {
                let arg_name = self.expect_ident()?;

                self.expect(Token::Colon)?;

                let ty = self.parse_type()?;

                let arg_ref = params.len();

                if args.insert(arg_name.clone(), arg_ref).is_some() {
                    return Err(ParseError::new(format!("duplicate argument: {arg_name}")));
                }

                params.push(ty);

                if self.peek() == Some(&Token::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }
        }

        self.expect(Token::RParen)?;
        self.expect(Token::Colon)?;

        let return_type = self.parse_type()?;

        self.expect(Token::LBrace)?;

        Ok((name, params, args, return_type))
    }

    fn parse_locals(&mut self) -> ParseResult<(Vec<Type>, HashMap<String, LocalRef>)> {
        self.expect(Token::Locals)?;
        // self.expect(Token::Ident("locals".into()))?;

        self.expect(Token::LBracket)?;

        let mut locals = Vec::new();
        let mut local_names = HashMap::new();

        if self.peek() != Some(&Token::RBracket) {
            loop {
                let local_name = self.expect_ident()?;

                self.expect(Token::Colon)?;

                let ty = self.parse_type()?;

                let local_ref = locals.len() as LocalRef;

                if local_names.insert(local_name.clone(), local_ref).is_some() {
                    return Err(ParseError::new(format!("duplicate local: {local_name}")));
                }

                locals.push(ty);

                if self.peek() == Some(&Token::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }
        }

        self.expect(Token::RBracket)?;

        Ok((locals, local_names))
    }
}

#[derive(Default)]
struct FunctionParseContext {
    args: HashMap<String, ArgRef>,
    locals: HashMap<String, LocalRef>,
    labels: HashMap<String, LabelRef>,
    next_label: LabelRef,
}

impl FunctionParseContext {
    fn get_label(&mut self, name: &str) -> LabelRef {
        if let Some(&label) = self.labels.get(name) {
            return label;
        }

        let label = self.next_label;
        self.next_label += 1;

        self.labels.insert(name.to_string(), label);

        label
    }

    fn resolve_arg(&self, name: &str) -> ParseResult<ArgRef> {
        self.args
            .get(name)
            .copied()
            .ok_or_else(|| ParseError::new(format!("unknown argument: {name}")))
    }

    fn resolve_local(&self, name: &str) -> ParseResult<LocalRef> {
        self.locals
            .get(name)
            .copied()
            .ok_or_else(|| ParseError::new(format!("unknown local: {name}")))
    }
}

impl Parser {
    fn parse_instruction(
        &mut self,
        ctx: &mut FunctionParseContext,
        function_names: &HashMap<String, FunctionRef>,
        pending_calls: &mut Vec<(FunctionRef, usize, usize, String)>,
        function_ref: FunctionRef,
        block_index: usize,
        instruction_index: usize,
    ) -> ParseResult<Instruction> {
        let token = self
            .advance()
            .ok_or_else(|| ParseError::new("unexpected end of input"))?;

        let instruction = match token {
            Token::Ldarg => Instruction::LoadArg(self.parse_arg_ref(ctx)?),

            Token::Ldloc => Instruction::LoadLocal(self.parse_local_ref(ctx)?),

            Token::Stloc => Instruction::StoreLocal(self.parse_local_ref(ctx)?),

            Token::Ldc => {
                let ty = self.parse_type()?;
                let constant = self.parse_constant()?;

                Instruction::LoadConstant(constant, ty)
            }

            Token::Call => {
                let name = self.expect_ident()?;

                if let Some(&function) = function_names.get(&name) {
                    Instruction::Call(function)
                } else {
                    pending_calls.push((function_ref, block_index, instruction_index, name));

                    Instruction::Call(u32::MAX)
                }
            }

            Token::Br => {
                let name = self.expect_ident()?;
                Instruction::Goto(ctx.get_label(&name))
            }

            Token::Brtrue => {
                let name = self.expect_ident()?;
                Instruction::GotoIfTrue(ctx.get_label(&name))
            }

            Token::Brfalse => {
                let name = self.expect_ident()?;
                Instruction::GotoIfFalse(ctx.get_label(&name))
            }

            Token::Ret => Instruction::Ret,
            Token::Assert => Instruction::Assert,

            Token::Pop => Instruction::Pop,
            Token::Dup => Instruction::Dup,
            Token::Swp => Instruction::Swp,
            Token::Over => Instruction::Over,
            Token::Nip => Instruction::Nip,
            Token::Tuck => Instruction::Tuck,
            Token::Rot => Instruction::Rot,

            Token::Pick => Instruction::Pick(self.expect_int()? as usize),

            Token::Roll => Instruction::Roll(self.expect_int()? as usize),

            Token::Iadd => Instruction::IAdd,
            Token::Isub => Instruction::ISub,
            Token::Imul => Instruction::IMul,
            Token::Idiv => Instruction::IDiv,
            Token::Udiv => Instruction::UDiv,
            Token::Irem => Instruction::IRem,
            Token::Urem => Instruction::URem,
            Token::IAbs => Instruction::IAbs,
            Token::INeg => Instruction::INeg,

            Token::IAnd => Instruction::IAnd,
            Token::IOr => Instruction::IOr,
            Token::IXor => Instruction::IXor,
            Token::INot => Instruction::INot,
            Token::IShl => Instruction::IShl,
            Token::IShr => Instruction::IShr,
            Token::ISar => Instruction::ISar,

            Token::Ieq
            | Token::Ine
            | Token::Islt
            | Token::Isle
            | Token::Isgt
            | Token::Isge
            | Token::Iult
            | Token::Iule
            | Token::Iugt
            | Token::Iuge => {
                self.cursor -= 1;
                Instruction::ICmp(self.parse_predicate()?)
            }

            Token::Fadd => Instruction::FAdd,
            Token::Fsub => Instruction::FSub,
            Token::Fmul => Instruction::FMul,
            Token::Fdiv => Instruction::FDiv,
            Token::Frem => Instruction::FRem,
            Token::FAbs => Instruction::FAbs,
            Token::FNeg => Instruction::FNeg,

            Token::Feq
            | Token::Fne
            | Token::Folt
            | Token::Fole
            | Token::Fogt
            | Token::Foge
            | Token::Fult
            | Token::Fule
            | Token::Fugt
            | Token::Fuge => {
                self.cursor -= 1;
                Instruction::FCmp(self.parse_float_predicate()?)
            }

            Token::Cast => Instruction::Cast(self.parse_type()?),

            token => {
                return Err(ParseError::new(format!(
                    "unsupported instruction: {token:?}"
                )));
            }
        };

        Ok(instruction)
    }
}

impl Parser {
    fn parse_function(
        &mut self,
        function_ref: FunctionRef,
        function_names: &HashMap<String, FunctionRef>,
        pending_calls: &mut Vec<(FunctionRef, usize, usize, String)>,
    ) -> ParseResult<Function> {
        let (name, params, args, return_type) = self.parse_function_signature()?;

        let (locals, local_names) = if self.peek() == Some(&Token::Locals) {
            self.parse_locals()?
        } else {
            (Vec::new(), HashMap::new())
        };

        let mut function = Function::new(&name, &params, return_type).with_locals(&locals);

        let mut ctx = FunctionParseContext {
            args,
            locals: local_names,
            ..Default::default()
        };

        while self.peek() != Some(&Token::RBrace) {
            let label_name = self.expect_ident()?;
            self.expect(Token::Colon)?;

            let label = ctx.get_label(&label_name);

            let mut block = BasicBlock::new(label);

            let block_index = function.blocks.len();

            while let Some(token) = self.peek() {
                if matches!(token, Token::RBrace) {
                    break;
                }

                if matches!(token, Token::Ident(_))
                    && self.tokens.get(self.cursor + 1) == Some(&Token::Colon)
                {
                    break;
                }

                let instruction_index = block.instructions.len();

                let instruction = self.parse_instruction(
                    &mut ctx,
                    function_names,
                    pending_calls,
                    function_ref,
                    block_index,
                    instruction_index,
                )?;

                block.instructions.push(instruction);
            }

            function.blocks.push(block);
        }

        self.expect(Token::RBrace)?;

        Ok(function)
    }
}

impl Parser {
    pub fn parse_module(mut self, name: &str) -> ParseResult<Module> {
        let mut module = Module::new(name);

        let mut function_names = HashMap::<String, FunctionRef>::new();

        let mut pending_calls = Vec::<(FunctionRef, usize, usize, String)>::new();

        let mut function_ref: FunctionRef = 0;

        while self.peek().is_some() {
            if self.peek() != Some(&Token::Function) {
                return Err(ParseError::new(format!(
                    "expected function, got {:?}",
                    self.peek()
                )));
            }

            let saved_cursor = self.cursor;

            self.advance();

            let function_name = self.expect_ident()?;

            self.cursor = saved_cursor;

            if function_names.contains_key(&function_name) {
                return Err(ParseError::new(format!(
                    "duplicate function: {function_name}"
                )));
            }

            function_names.insert(function_name.clone(), function_ref);

            let function =
                self.parse_function(function_ref, &function_names, &mut pending_calls)?;

            module.set_function(function_ref, function);

            function_ref += 1;
        }

        // Resolve calls.
        for (caller_ref, block_index, instruction_index, name) in pending_calls {
            let callee_ref = function_names
                .get(&name)
                .copied()
                .ok_or_else(|| ParseError::new(format!("unknown function: {name}")))?;

            let function = module
                .get_function_mut(caller_ref)
                .ok_or_else(|| ParseError::new("invalid caller function reference"))?;

            let block = function
                .blocks
                .get_mut(block_index)
                .ok_or_else(|| ParseError::new("invalid block reference"))?;

            let instruction = block
                .instructions
                .get_mut(instruction_index)
                .ok_or_else(|| ParseError::new("invalid instruction reference"))?;

            *instruction = Instruction::Call(callee_ref);
        }

        Ok(module)
    }
}

#[cfg(test)]
mod tests {
    use crate::lexer::Lexer;
    use crate::parser::*;

    fn parse(source: &str) -> ParseResult<Module> {
        let tokens = Lexer::new(source)
            .tokenize()
            .map_err(|e| ParseError::new(format!("{e:?}")))?;

        Parser::new(tokens).parse_module("test")
    }

    #[test]
    fn parses_named_arguments() {
        let module = parse(
            r#"
            function add(lhs: i32, rhs: i32): i32 {
            entry:
                ldarg #lhs
                ldarg #rhs
                iadd
                ret
            }
            "#,
        )
        .unwrap();

        let function = module.get_function(0).unwrap();

        assert_eq!(function.params, vec![Type::Int32, Type::Int32]);

        assert_eq!(
            function.blocks[0].instructions,
            vec![
                Instruction::LoadArg(0),
                Instruction::LoadArg(1),
                Instruction::IAdd,
                Instruction::Ret,
            ],
        );
    }

    #[test]
    fn parses_named_locals() {
        let module = parse(
            r#"
            function test(x: i32): i32 {
                locals [tmp: i32]

            entry:
                ldarg #x
                stloc %tmp
                ldloc %tmp
                ret
            }
            "#,
        )
        .unwrap();

        let function = module.get_function(0).unwrap();

        assert_eq!(function.locals, vec![Type::Int32]);

        assert_eq!(
            function.blocks[0].instructions,
            vec![
                Instruction::LoadArg(0),
                Instruction::StoreLocal(0),
                Instruction::LoadLocal(0),
                Instruction::Ret,
            ],
        );
    }

    #[test]
    fn parses_named_blocks() {
        let module = parse(
            r#"
            function test(x: i32): void {
            entry:
                ldarg #x
                brtrue yes
                br end

            yes:
                ret

            end:
                ret
            }
            "#,
        )
        .unwrap();

        let function = module.get_function(0).unwrap();

        assert_eq!(function.blocks.len(), 3);

        assert_eq!(function.blocks[0].label, 0);
        assert_eq!(function.blocks[1].label, 1);
        assert_eq!(function.blocks[2].label, 2);

        assert_eq!(
            function.blocks[0].instructions,
            vec![
                Instruction::LoadArg(0),
                Instruction::GotoIfTrue(1),
                Instruction::Goto(2),
            ],
        );
    }

    #[test]
    fn parses_named_function_calls() {
        let module = parse(
            r#"
            function caller(x: i32): i32 {
            entry:
                ldarg #x
                call callee
                ret
            }

            function callee(x: i32): i32 {
            entry:
                ldarg #x
                ret
            }
            "#,
        )
        .unwrap();

        let caller = module.get_function(0).unwrap();

        assert_eq!(
            caller.blocks[0].instructions,
            vec![
                Instruction::LoadArg(0),
                Instruction::Call(1),
                Instruction::Ret,
            ],
        );
    }

    #[test]
    fn parses_recursive_call() {
        let module = parse(
            r#"
            function fib(n: i32): i32 {
            entry:
                ldarg #n
                call fib
                ret
            }
            "#,
        )
        .unwrap();

        let function = module.get_function(0).unwrap();

        assert_eq!(
            function.blocks[0].instructions,
            vec![
                Instruction::LoadArg(0),
                Instruction::Call(0),
                Instruction::Ret,
            ],
        );
    }

    #[test]
    fn rejects_unknown_argument() {
        let result = parse(
            r#"
            function test(x: i32): i32 {
            entry:
                ldarg #unknown
                ret
            }
            "#,
        );

        assert!(result.is_err());
    }

    #[test]
    fn rejects_unknown_local() {
        let result = parse(
            r#"
            function test(): i32 {
                locals [tmp: i32]

            entry:
                ldloc %unknown
                ret
            }
            "#,
        );

        assert!(result.is_err());
    }

    #[test]
    fn rejects_unknown_function() {
        let result = parse(
            r#"
            function test(): void {
            entry:
                call unknown
                ret
            }
            "#,
        );

        assert!(result.is_err());
    }

    #[test]
    fn rejects_duplicate_arguments() {
        let result = parse(
            r#"
            function test(x: i32, x: i64): void {
            entry:
                ret
            }
            "#,
        );

        assert!(result.is_err());
    }

    #[test]
    fn rejects_duplicate_locals() {
        let result = parse(
            r#"
            function test(): void {
                .locals [x: i32, x: i64]

            entry:
                ret
            }
            "#,
        );

        assert!(result.is_err());
    }

    #[test]
    fn rejects_duplicate_functions() {
        let result = parse(
            r#"
            function foo(): void {
            entry:
                ret
            }

            function foo(): void {
            entry:
                ret
            }
            "#,
        );

        assert!(result.is_err());
    }
}
