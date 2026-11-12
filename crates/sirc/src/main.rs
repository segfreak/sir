
use std::{
    fs::File,
    io::BufWriter,
    path::PathBuf,
    str::FromStr,
};

use clap::Parser;
use fear::types::OptLevel;
use sir_reader::{lexer, parser};
use target_lexicon::Triple;

#[derive(Debug, Parser)]
#[command(name = "sirc", version, about = "SIR compiler")]
struct Cli {
    #[arg(help = "Input module")]
    input: PathBuf,

    #[arg(
        short = 'o',
        long = "output",
        help = "Output object file"
    )]
    output_path: Option<PathBuf>,

    #[arg(long = "opt", value_enum)]
    opt_level: Option<OptLevel>,

    #[arg(long = "emit-fear", help = "Emit fear binary module")]
    emit_fear: bool,

    #[arg(
        short = 't',
        long = "triple",
        value_parser = |s: &str| {
            Triple::from_str(s).map_err(|e| e.to_string())
        },
        help = "Target platform triple (defaults to host)"
    )]
    target_triple: Option<Triple>,

    #[arg(long = "pic", help = "Enable position-independent code generation")]
    pic: bool,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let ext = match cli.emit_fear {
        true => "bin",
        false => "o"
    };

    let output = cli
        .output_path
        .unwrap_or_else(|| cli.input.with_extension(ext));

    let target_triple = cli.target_triple.unwrap_or_else(Triple::host);
    let opt_level = cli.opt_level.unwrap_or(OptLevel::Default);

    log::debug!("input:  {}", cli.input.display());
    log::debug!("output: {}", output.display());
    log::debug!("opt:    {:?}", opt_level);
    log::debug!("target: {}", target_triple);

    let source = std::fs::read_to_string(&cli.input)?;

    let tokens = lexer::Lexer::new(&source).tokenize()
        .map_err(|e| anyhow::anyhow!("lexing failed: {e}"))?;

    let module_name = cli.input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("module");

    let module = parser::Parser::new(tokens)
        .parse_module(module_name)
        .map_err(|e| anyhow::anyhow!("parsing failed: {e}"))?;

    let fear_module = sir_codegen_fear::lower_module(&module);

    let file = File::create(&output)?;
    let writer = BufWriter::new(file);

    if cli.emit_fear {
        fear::binary::write(&fear_module, writer).expect("failed to write binary module");
        return Ok(());
    }

    fear::compiler::compile_module(
        &fear_module,
        &fear::compiler::CompilerConfig::setup(
            fear::compiler::OutputType::Object,
            target_triple,
            opt_level,
            cli.pic,
            None,
        ),
        writer,
    ).expect("failed to compile module");

    Ok(())
}
