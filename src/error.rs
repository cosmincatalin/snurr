pub type Result<T> = std::result::Result<T, Error>;

/// Snurr Errors
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("BPMN type {0} missing id")]
    MissingId(String),

    #[error("{0} has no output. (Used correct name or id?)")]
    MissingOutput(String),

    #[error("{0} has no implementation")]
    MissingImplementation(String),

    #[error("Missing implementations {0}")]
    MissingImplementations(String),

    #[error("{0} has no default flow")]
    MissingDefault(String),

    #[error("could not find BPMN data with id {0}")]
    MisssingBpmnData(String),

    #[error("could not find process data with id {0}")]
    MissingProcessData(String),

    #[error("missing definitions id")]
    MissingDefinitionsId,

    #[error("sequenceFlow missing targetRef")]
    MissingTargetRef,

    #[error("type {0} not implemented")]
    TypeNotImplemented(String),

    #[error("could not find {0} boundary symbol attached to {1}")]
    MissingBoundary(String, String),

    #[error("{0} could not find {1}")]
    MissingIntermediateEvent(String, String),

    #[error("missing intermediate throw event name on {0}")]
    MissingIntermediateThrowEventName(String),

    #[error("missing intermediate catch event symbol {0} with name {1}")]
    MissingIntermediateCatchEvent(String, String),

    #[error("missing end event")]
    MissingEndEvent,

    #[error("missing start event")]
    MissingStartEvent,

    #[error("couldn't extract process result")]
    NoProcessResult,

    #[error("{0} not supported")]
    NotSupported(String),

    #[error("{0}")]
    BpmnRequirement(String),

    #[error("Process execution error: {0}")]
    ProcessExecution(#[source] Box<dyn std::error::Error + Send + Sync>),

    #[error("{0}")]
    Builder(String),

    #[error(transparent)]
    File(#[from] quick_xml::Error),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Utf8(#[from] std::str::Utf8Error),
}

// BpmnRequirement
pub(crate) const AT_LEAST_TWO_OUTGOING: &str =
    "Event gateway must have at least two outgoing sequence flows";
pub(crate) const ONLY_ONE_START_EVENT: &str = "There can only be one start event of type none";

// Builder
pub(crate) const FUNC_MAP_ERROR_MSG: &str = "Handlermap has already been consumed";
pub(crate) const BUILD_PROCESS_ERROR_MSG: &str = "Couldn't build process";
