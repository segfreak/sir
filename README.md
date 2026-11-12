# StackIR (SIR)

**Stack Intermediate Representation**

StackIR is a lightweight, typed, stack-based intermediate representation designed for compiler infrastructure.

It provides a textual representation of functions, basic blocks, instructions, and control flow. Its stack-oriented instruction model makes it suitable as a frontend IR that can be lowered into SSA or other compiler representations.

## 1. Overview

StackIR is organized around the following concepts:

- **Module** — a collection of functions.
- **Function** — a typed function with parameters, local variables, and basic blocks.
- **Basic block** — a labeled sequence of instructions.
- **Instruction** — an operation that manipulates the evaluation stack.
- **Argument reference** — identifies a function parameter.
- **Local reference** — identifies a local variable.

StackIR uses explicit types, named arguments, named local variables, and named basic blocks.

### Example

```sir
function add(lhs: i32, rhs: i32): i32 {
entry:
    ldarg #lhs
    ldarg #rhs
    iadd
    ret
}
```

This function loads two arguments, adds them, and returns the result.

## 2. Lexical structure

### 2.1. Whitespace

Whitespace separates tokens and is ignored by the lexer.

Supported whitespace characters include spaces, tabs, carriage returns, and newlines.

### 2.2. Comments

StackIR supports two single-line comment styles:

```sir
;; This is a comment.

// This is also a comment.
```

Comments extend to the end of the current line.

### 2.3. Identifiers

Identifiers consist of ASCII letters, digits, and underscores.

An identifier must begin with an ASCII letter or underscore.

```sir
entry
main
calculate_sum
arg0
tmp_1
```

Identifiers are case-sensitive.

### 2.4. Special identifiers

Identifiers beginning with a dot are supported:

```sir
.entry
.loop
.exit
```

The dot-prefixed form is lexed as an identifier and is not equivalent to a keyword.

## 3. Types

StackIR supports the following primitive types:

| Type | Description |
|---|---|
| `void` | No return value |
| `i1` | 1-bit integer / boolean |
| `i8` | 8-bit integer |
| `i16` | 16-bit integer |
| `i32` | 32-bit integer |
| `i64` | 64-bit integer |
| `f32` | 32-bit floating-point |
| `f64` | 64-bit floating-point |

Types are specified in function signatures, local declarations, and constant instructions.

Example:

```sir
function calculate(a: i32, b: f64): f64 {
entry:
    ldc f64 2.5
    ret
}
```

## 4. Integer literals

Integer literals support multiple radices.

| Format | Prefix | Example |
|---|---|---|
| Decimal | None | `42` |
| Hexadecimal | `0x`, `0X` | `0xFF` |
| Binary | `0b`, `0B` | `0b1010` |
| Octal | `0o`, `0O` | `0o755` |
| Implicit octal | Leading zero | `0777` |

Negative integer literals are supported:

```sir
ldc i32 -42
ldc i32 -0x10
```

Integer literals are represented using signed 64-bit values.

Values outside the supported range produce a lexical error.

## 5. Floating-point literals

Floating-point literals support decimal notation and scientific notation.

Examples:

```sir
ldc f32 1.5
ldc f64 3.141592
ldc f64 -0.125
ldc f64 1.25e10
ldc f64 2.5e-3
```

A floating-point literal must contain either a fractional component or an exponent to be recognized as a floating-point value.

Floating-point values are represented internally as `f64`.

## 6. References

StackIR uses explicit prefixes to distinguish different reference types.

### 6.1. Argument references

The `#` prefix identifies function arguments.

Arguments can be referenced by name or numeric index.

```sir
ldarg #x
ldarg #0
```

Named references are resolved against the function signature.

Numeric references represent zero-based argument indices.

### 6.2. Local references

The `%` prefix identifies local variables.

Locals can also be referenced by name or numeric index.

```sir
ldloc %tmp
stloc %tmp

ldloc %0
stloc %1
```

Numeric references represent zero-based local indices.

## 7. Modules and functions

A module consists of one or more function definitions.

### 7.1. Function declaration

General syntax:

```sir
function <name>(<parameters>): <return_type> {
    <function_body>
}
```

A function consists of:

1. Function name.
2. Parameter list.
3. Return type.
4. Optional local variable declaration.
5. One or more basic blocks.

### 7.2. Function parameters

Parameters follow the syntax:

```sir
<name>: <type>
```

Multiple parameters are separated by commas.

Example:

```sir
function multiply(a: i32, b: i32): i32 {
entry:
    ldarg #a
    ldarg #b
    imul
    ret
}
```

Duplicate parameter names are rejected.

### 7.3. Local variables

Local variables are declared using the `locals` keyword.

Syntax:

```sir
locals [
    <name>: <type>,
    ...
]
```

Example:

```sir
function calculate(x: i32): i32 {
    locals [
        tmp: i32,
        result: i32
    ]

entry:
    ldarg #x
    stloc %tmp

    ldloc %tmp
    ldc i32 10
    iadd
    stloc %result

    ldloc %result
    ret
}
```

Local variable declarations are optional.

Duplicate local names are rejected.

## 8. Basic blocks

A basic block is identified by a unique label.

Syntax:

```sir
<label>:
    <instruction>
    <instruction>
```

Example:

```sir
function test(x: i32): i32 {
entry:
    ldarg #x
    brtrue nonzero
    br zero

nonzero:
    ldc i32 1
    ret

zero:
    ldc i32 0
    ret
}
```

Branch instructions reference labels by name.

Labels are resolved throughout the function, allowing forward references.

The parser assigns numeric label references in the order labels are first encountered.

## 9. Instruction set

Instructions operate on an evaluation stack.

### 9.1. Data movement

| Instruction | Description |
|---|---|
| `ldarg #ref` | Load function argument |
| `ldloc %ref` | Load local variable |
| `stloc %ref` | Store value into local variable |
| `ldc type value` | Load constant |

Examples:

```sir
ldarg #0
ldloc %tmp
stloc %tmp

ldc i32 42
ldc f64 3.14
```

### 9.2. Integer arithmetic

| Instruction | Description |
|---|---|
| `iadd` | Integer addition |
| `isub` | Integer subtraction |
| `imul` | Integer multiplication |
| `idiv` | Signed integer division |
| `udiv` | Unsigned integer division |
| `irem` | Signed integer remainder |
| `urem` | Unsigned integer remainder |
| `iabs` | Integer absolute value |
| `ineg` | Integer negation |

### 9.3. Integer bitwise operations

| Instruction | Description |
|---|---|
| `iand` | Bitwise AND |
| `ior` | Bitwise OR |
| `ixor` | Bitwise XOR |
| `inot` | Bitwise NOT |
| `ishl` | Logical left shift |
| `ishr` | Logical right shift |
| `isar` | Arithmetic right shift |

### 9.4. Integer comparisons

Integer comparison instructions produce boolean results.

| Instruction | Description |
|---|---|
| `eq` | Equal |
| `ne` | Not equal |
| `slt` | Signed less than |
| `sle` | Signed less than or equal |
| `sgt` | Signed greater than |
| `sge` | Signed greater than or equal |
| `ult` | Unsigned less than |
| `ule` | Unsigned less than or equal |
| `ugt` | Unsigned greater than |
| `uge` | Unsigned greater than or equal |

### 9.5. Floating-point arithmetic

| Instruction | Description |
|---|---|
| `fadd` | Floating-point addition |
| `fsub` | Floating-point subtraction |
| `fmul` | Floating-point multiplication |
| `fdiv` | Floating-point division |
| `frem` | Floating-point remainder |
| `fabs` | Floating-point absolute value |
| `fneg` | Floating-point negation |

### 9.6. Floating-point comparisons

| Instruction | Description |
|---|---|
| `feq` | Equal |
| `fne` | Not equal |
| `folt` | Ordered less than |
| `fole` | Ordered less than or equal |
| `fogt` | Ordered greater than |
| `foge` | Ordered greater than or equal |
| `fult` | Unordered less than |
| `fule` | Unordered less than or equal |
| `fugt` | Unordered greater than |
| `fuge` | Unordered greater than or equal |

### 9.7. Type conversion

```sir
cast <type>
```

Converts the top stack value to the specified type.

Example:

```sir
ldc i32 42
cast f64
ret
```

### 9.8. Assertions

```sir
assert
```

Checks the condition represented by the top stack value.

### 9.9. Control flow

| Instruction | Description |
|---|---|
| `ret` | Return from function |
| `br label` | Unconditional branch |
| `brtrue label` | Branch if condition is true |
| `brfalse label` | Branch if condition is false |
| `call function` | Call another function |

Examples:

```sir
br loop

brtrue exit

brfalse fallback

call calculate
```

Function calls are resolved by function name.

Forward function references and recursive calls are supported.

### 9.10. Stack manipulation

| Instruction | Description |
|---|---|
| `pop` | Remove top stack element |
| `dup` | Duplicate top stack element |
| `swp` | Swap top two elements |
| `over` | Copy second element to top |
| `nip` | Remove second element |
| `tuck` | Insert top element below second |
| `rot` | Rotate top three elements |
| `pick n` | Copy element at depth `n` |
| `roll n` | Move element at depth `n` to top |

`pick` and `roll` accept integer depth operands.

Example:

```sir
ldc i32 10
ldc i32 20
dup
iadd
ret
```

## 10. Function calls

Functions are referenced by name.

Example:

```sir
function add(a: i32, b: i32): i32 {
entry:
    ldarg #a
    ldarg #b
    iadd
    ret
}

function main(): i32 {
entry:
    ldc i32 10
    ldc i32 20
    call add
    ret
}
```

Function references are resolved after parsing the module.

This allows:

- Calls to previously declared functions.
- Calls to functions declared later.
- Recursive function calls.

The parser stores unresolved calls temporarily and resolves them after all function definitions have been processed.

## 11. Complete example

The following example implements an iterative Fibonacci function.

```sir
function fibonacci(n: i32): i32 {
    locals [
        a: i32,
        b: i32,
        i: i32,
        tmp: i32
    ]

entry:
    ldc i32 0
    stloc %a

    ldc i32 1
    stloc %b

    ldc i32 0
    stloc %i

    br loop

loop:
    ldloc %i
    ldarg #n
    slt
    brfalse exit

    ldloc %b
    stloc %tmp

    ldloc %a
    ldloc %b
    iadd
    stloc %b

    ldloc %tmp
    stloc %a

    ldloc %i
    ldc i32 1
    iadd
    stloc %i

    br loop

exit:
    ldloc %a
    ret
}
```

## 12. Parsing and validation

The parser performs the following operations:

- Tokenizes source text.
- Parses function signatures.
- Resolves named arguments.
- Resolves named local variables.
- Parses basic blocks.
- Assigns numeric references to labels.
- Resolves function calls.
- Constructs the module representation.

### 12.1. Supported validation

The current parser rejects:

- Invalid argument references.
- Unknown argument names.
- Invalid local references.
- Unknown local names.
- Duplicate argument names.
- Duplicate local names.
- Duplicate function names.
- Unknown function calls.
- Unexpected tokens.
- Unsupported instructions.

### 12.2. Error reporting

Lexical errors contain source line and column information.

Examples:

```text
1:5: unexpected character '@'
2:12: expected integer digits
3:8: integer literal out of range
```

Parser errors currently contain descriptive messages.

## 13. Grammar

The following EBNF describes the core textual syntax.

```ebnf
module          = { function } ;

function        = "function", identifier,
                  "(", [ parameters ], ")",
                  ":", type,
                  "{",
                  [ locals ],
                  { basic_block },
                  "}" ;

parameters      = parameter, { ",", parameter } ;
parameter       = identifier, ":", type ;

locals          = "locals", "[",
                  [ local, { ",", local } ],
                  "]" ;

local           = identifier, ":", type ;

basic_block     = identifier, ":", { instruction } ;

type            = "void"
                | "i1"
                | "i8"
                | "i16"
                | "i32"
                | "i64"
                | "f32"
                | "f64" ;

argument_ref    = "#", ( identifier | integer ) ;
local_ref       = "%", ( identifier | integer ) ;

instruction     = "ldarg", argument_ref
                | "ldloc", local_ref
                | "stloc", local_ref
                | "ldc", type, constant
                | "call", identifier
                | "br", identifier
                | "brtrue", identifier
                | "brfalse", identifier
                | "ret"
                | "assert"
                | integer_instruction
                | float_instruction
                | comparison_instruction
                | stack_instruction
                | "cast", type ;

constant        = integer | float ;

integer         = [ "-" ], integer_literal ;
float           = [ "-" ], float_literal ;
identifier      = letter, { letter | digit | "_" } ;
```

The grammar above is a high-level description. Exact tokenization rules, numeric formats, and instruction operands are defined by the lexer and parser implementation.

## 14. Design principles

StackIR follows several design principles:

- **Simplicity:** a compact instruction set and straightforward textual syntax.
- **Explicit typing:** types are declared in signatures, locals, and constants.
- **Named references:** arguments, locals, and basic blocks can be referenced symbolically.
- **Deterministic references:** named entities are mapped to numeric references during parsing.
- **Modularity:** functions are independently represented and can be processed separately.
- **Lowerability:** the stack-based representation can be transformed into SSA or another compiler IR.

## 15. Project status

StackIR currently provides:

- Lexer.
- Token representation.
- Textual parser.
- Module and function parsing.
- Named argument and local resolution.
- Basic block parsing.
- Function call resolution.
- Integer and floating-point instruction parsing.
- Unit tests for core parser functionality.

The following areas are not fully specified by the current parser implementation and require additional semantic validation:

- Operand stack typing and stack consistency.
- Integer overflow and division semantics.
- Floating-point exception and NaN semantics.
- Exact behavior of stack manipulation instructions.
- Branch target validation.
- Return type and return stack validation.
- Control-flow termination rules.
- Function call signature validation.

These checks should be performed by a subsequent semantic validation pass.

---

**StackIR — Stack Intermediate Representation**

A compact stack-based IR designed to serve as a frontend representation for compiler pipelines.