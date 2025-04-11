use nom::IResult;
use nom_language::error::VerboseError;

pub type SixuResult<I, O> = IResult<I, O, VerboseError<I>>;
