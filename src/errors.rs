use thiserror::Error;

#[derive(Debug, Error)]
pub enum InvalidInstructionError {
    #[error("Invalid instruction: {0}")]
    InvalidInstruction(String),
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Failed to parse TOML file.")]
    ParserError(),

    #[error("Failed to find config.")]
    FileNotFound(),

    #[error("Missing '{0}' table in config.")]
    MissingTable(String),

    #[error("Missing '{0}' value in table [{1}].")]
    MissingValue(String, String),

    #[error("Missing '{0}' '{1}' value for key '{3}' in table [{3}].")]
    MissingValueForKey(String, String, String, String),

    #[error("Invalid key '{0}' value in table [{1}].")]
    InvalidKey(String, String),

    #[error("Invalid value '{0}' value in table [{1}].")]
    InvalidValue(String, String),

    #[error("Invalid type for '{0}' value in table [{1}].")]
    InvalidType(String, String),
}

#[derive(Debug, Error)]
pub enum ExecutionEngineError {
    #[error("Expected arg{0} to be '{1}', but it was {2}")]
    ExpectationError(u8, String, String),

    #[error("Invalid instruction!")]
    InvalidInstruction(),
}

#[derive(Debug, Error)]
pub enum KernelError {
    #[error("Error converting bytes")]
    ConversionError(),

    #[error("Invalid file mode:{0}")]
    InvalidFileMode(String),

    #[error("Failed to read from {0}: {1}")]
    ReadFail(String, String),

    #[error("Invalid file descriptor!")]
    InvalidFileDescriptor(),

    #[error("Program exited!")]
    Exit(u8),
}

#[derive(Error, Debug)]
pub enum ExecutionError {
    #[error(transparent)]
    Execution(#[from] ExecutionEngineError),

    #[error(transparent)]
    Kernel(#[from] KernelError),
}

#[derive(Debug, Error)]
pub enum MachineError {
    #[error(transparent)]
    ExecutionError(#[from] ExecutionError),

    #[error("Invalid instruction!")]
    InvalidInstruction(),
}

#[derive(Debug, Error)]
pub enum RegisterFileError {
    #[error("{0} not found")]
    RegisterNotFound(String),
}
