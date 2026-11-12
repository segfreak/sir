#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Token {
    Function,

    Star,
    I1,
    I8,
    I32,
    I64,

    IntLiteral(i64),
    Ident(String),

    ArgRef(Box<Token>),   // #X
    LocalRef(Box<Token>), // $X
    BlockRef(Box<Token>), // .X

    LBrace,
    RBrace,
    LParen,
    RParen,
    Colon,

    Ldarg,
    Ldc,
    Ldloc,
    Stloc,
    Iadd,
    Isub,
    Imul,
    Idiv,
    Udiv,
    Irem,
    Urem,
    IAbs,

    INeg,
    IAnd,
    IOr,
    IXor,
    INot,
    IShl,
    IShr,
    ISar,

    Ieq,
    Ine,
    Islt,
    Isle,
    Isgt,
    Isge,

    Cast,

    Assert,

    Ret,
    Br,
    Brtrue,
    Brfalse,

    Pop,
    Dup,
    Swp,
    Over,
    Nip,
    Tuck,
    Rot,
    Pick,
    Roll,
}
