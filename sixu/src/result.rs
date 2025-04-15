use nom::IResult;
use nom_language::error::VerboseError;

pub type ParseResult<I, O> = IResult<I, O, VerboseError<I>>;
